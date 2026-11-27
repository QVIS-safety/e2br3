--
-- PostgreSQL database dump
--



SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;

SET default_tablespace = '';

--
-- Name: app_settings app_settings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_settings
    ADD CONSTRAINT app_settings_pkey PRIMARY KEY (organization_id, key);


--
-- Name: audit_logs audit_logs_hash_version_valid; Type: CHECK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.audit_logs
    ADD CONSTRAINT audit_logs_hash_version_valid CHECK ((hash_version = ANY (ARRAY[1, 2]))) NOT VALID;


--
-- Name: audit_logs audit_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_pkey PRIMARY KEY (id);


--
-- Name: authorization_audit_events authorization_audit_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_audit_events
    ADD CONSTRAINT authorization_audit_events_pkey PRIMARY KEY (id);


--
-- Name: authorization_catalog_state authorization_catalog_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_catalog_state
    ADD CONSTRAINT authorization_catalog_state_pkey PRIMARY KEY (singleton);


--
-- Name: authorization_grant_catalog authorization_grant_catalog_pdf_order_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_grant_catalog
    ADD CONSTRAINT authorization_grant_catalog_pdf_order_key UNIQUE (pdf_order);


--
-- Name: authorization_grant_catalog authorization_grant_catalog_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_grant_catalog
    ADD CONSTRAINT authorization_grant_catalog_pkey PRIMARY KEY (grant_id);


--
-- Name: authorization_grant_role_classes authorization_grant_role_classes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_grant_role_classes
    ADD CONSTRAINT authorization_grant_role_classes_pkey PRIMARY KEY (grant_id, role_class);


--
-- Name: authorization_migration_reconciliations authorization_migration_reconciliations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_migration_reconciliations
    ADD CONSTRAINT authorization_migration_reconciliations_pkey PRIMARY KEY (user_id, organization_id);


--
-- Name: authorization_migration_rejections authorization_migration_rejections_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_migration_rejections
    ADD CONSTRAINT authorization_migration_rejections_pkey PRIMARY KEY (id);


--
-- Name: authorization_roles authorization_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_roles
    ADD CONSTRAINT authorization_roles_pkey PRIMARY KEY (id);


--
-- Name: autopsy_causes_of_death autopsy_causes_of_death_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.autopsy_causes_of_death
    ADD CONSTRAINT autopsy_causes_of_death_pkey PRIMARY KEY (id);


--
-- Name: case_e2b_field_notations case_e2b_field_notations_case_id_record_id_e2b_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_e2b_field_notations
    ADD CONSTRAINT case_e2b_field_notations_case_id_record_id_e2b_code_key UNIQUE NULLS NOT DISTINCT (case_id, record_id, e2b_code);


--
-- Name: case_e2b_field_notations case_e2b_field_notations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_e2b_field_notations
    ADD CONSTRAINT case_e2b_field_notations_pkey PRIMARY KEY (id);


--
-- Name: case_field_notations case_field_notations_case_id_record_id_field_path_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_field_notations
    ADD CONSTRAINT case_field_notations_case_id_record_id_field_path_key UNIQUE NULLS NOT DISTINCT (case_id, record_id, field_path);


--
-- Name: case_field_notations case_field_notations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_field_notations
    ADD CONSTRAINT case_field_notations_pkey PRIMARY KEY (id);


--
-- Name: case_submissions case_submissions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_submissions
    ADD CONSTRAINT case_submissions_pkey PRIMARY KEY (id);


--
-- Name: case_summary_information case_summary_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_summary_information
    ADD CONSTRAINT case_summary_information_pkey PRIMARY KEY (id);


--
-- Name: case_validation_summaries case_validation_summaries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_validation_summaries
    ADD CONSTRAINT case_validation_summaries_pkey PRIMARY KEY (case_id, appendix, page_id);


--
-- Name: case_versions case_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_versions
    ADD CONSTRAINT case_versions_pkey PRIMARY KEY (id);


--
-- Name: case_workflow_events case_workflow_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_workflow_events
    ADD CONSTRAINT case_workflow_events_pkey PRIMARY KEY (id);


--
-- Name: cases cases_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cases
    ADD CONSTRAINT cases_pkey PRIMARY KEY (id);


--
-- Name: controlled_terminology_terms controlled_terminology_terms_audit_id_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.controlled_terminology_terms
    ADD CONSTRAINT controlled_terminology_terms_audit_id_unique UNIQUE (audit_id);


--
-- Name: controlled_terminology_terms controlled_terminology_terms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.controlled_terminology_terms
    ADD CONSTRAINT controlled_terminology_terms_pkey PRIMARY KEY (id);


--
-- Name: controlled_terminology_terms controlled_terminology_terms_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.controlled_terminology_terms
    ADD CONSTRAINT controlled_terminology_terms_unique UNIQUE (dictionary, version, language, scope, code);


--
-- Name: dashboard_notices dashboard_notices_organization_id_notice_key_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dashboard_notices
    ADD CONSTRAINT dashboard_notices_organization_id_notice_key_key UNIQUE (organization_id, notice_key);


--
-- Name: dashboard_notices dashboard_notices_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dashboard_notices
    ADD CONSTRAINT dashboard_notices_pkey PRIMARY KEY (id);


--
-- Name: documents_held_by_sender documents_held_by_sender_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.documents_held_by_sender
    ADD CONSTRAINT documents_held_by_sender_pkey PRIMARY KEY (id);


--
-- Name: dosage_information dosage_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dosage_information
    ADD CONSTRAINT dosage_information_pkey PRIMARY KEY (id);


--
-- Name: drug_active_substances drug_active_substances_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_active_substances
    ADD CONSTRAINT drug_active_substances_pkey PRIMARY KEY (id);


--
-- Name: drug_device_characteristics drug_device_characteristics_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_device_characteristics
    ADD CONSTRAINT drug_device_characteristics_pkey PRIMARY KEY (id);


--
-- Name: drug_indications drug_indications_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_indications
    ADD CONSTRAINT drug_indications_pkey PRIMARY KEY (id);


--
-- Name: drug_information drug_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_information
    ADD CONSTRAINT drug_information_pkey PRIMARY KEY (id);


--
-- Name: drug_reaction_assessments drug_reaction_assessments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_reaction_assessments
    ADD CONSTRAINT drug_reaction_assessments_pkey PRIMARY KEY (id);


--
-- Name: e2b_code_lists e2b_code_lists_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e2b_code_lists
    ADD CONSTRAINT e2b_code_lists_pkey PRIMARY KEY (id);


--
-- Name: e_signatures e_signatures_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e_signatures
    ADD CONSTRAINT e_signatures_pkey PRIMARY KEY (id);


--
-- Name: fda_device_codes fda_device_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_device_codes
    ADD CONSTRAINT fda_device_codes_pkey PRIMARY KEY (id);


--
-- Name: fda_device_information fda_device_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_device_information
    ADD CONSTRAINT fda_device_information_pkey PRIMARY KEY (id);


--
-- Name: fda_hierarchical_code_lists fda_hierarchical_code_lists_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_hierarchical_code_lists
    ADD CONSTRAINT fda_hierarchical_code_lists_pkey PRIMARY KEY (id);


--
-- Name: iso_countries iso_countries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.iso_countries
    ADD CONSTRAINT iso_countries_pkey PRIMARY KEY (code);


--
-- Name: linked_report_numbers linked_report_numbers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.linked_report_numbers
    ADD CONSTRAINT linked_report_numbers_pkey PRIMARY KEY (id);


--
-- Name: literature_references literature_references_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.literature_references
    ADD CONSTRAINT literature_references_pkey PRIMARY KEY (id);


--
-- Name: meddra_terms meddra_terms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.meddra_terms
    ADD CONSTRAINT meddra_terms_pkey PRIMARY KEY (id);


--
-- Name: medical_history_episodes medical_history_episodes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.medical_history_episodes
    ADD CONSTRAINT medical_history_episodes_pkey PRIMARY KEY (id);


--
-- Name: mfds_product_substances mfds_product_substances_audit_id_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_product_substances
    ADD CONSTRAINT mfds_product_substances_audit_id_unique UNIQUE (audit_id);


--
-- Name: mfds_product_substances mfds_product_substances_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_product_substances
    ADD CONSTRAINT mfds_product_substances_pkey PRIMARY KEY (id);


--
-- Name: mfds_product_substances mfds_product_substances_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_product_substances
    ADD CONSTRAINT mfds_product_substances_unique UNIQUE NULLS NOT DISTINCT (item_seq, substance_code, material_sequence, total_amount_sequence, version);


--
-- Name: mfds_products mfds_products_audit_id_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_products
    ADD CONSTRAINT mfds_products_audit_id_unique UNIQUE (audit_id);


--
-- Name: mfds_products mfds_products_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_products
    ADD CONSTRAINT mfds_products_pkey PRIMARY KEY (id);


--
-- Name: mfds_products mfds_products_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_products
    ADD CONSTRAINT mfds_products_unique UNIQUE (item_seq, version);


--
-- Name: narrative_information narrative_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_information
    ADD CONSTRAINT narrative_information_pkey PRIMARY KEY (id);


--
-- Name: narrative_presaves narrative_presaves_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_presaves
    ADD CONSTRAINT narrative_presaves_pkey PRIMARY KEY (id);


--
-- Name: organization_as2_profiles organization_as2_profiles_organization_id_authority_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_as2_profiles
    ADD CONSTRAINT organization_as2_profiles_organization_id_authority_key UNIQUE (organization_id, authority);


--
-- Name: organization_as2_profiles organization_as2_profiles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_as2_profiles
    ADD CONSTRAINT organization_as2_profiles_pkey PRIMARY KEY (id);


--
-- Name: organization_policy_state organization_policy_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_policy_state
    ADD CONSTRAINT organization_policy_state_pkey PRIMARY KEY (organization_id);


--
-- Name: organizations organizations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_pkey PRIMARY KEY (id);


--
-- Name: other_case_identifiers other_case_identifiers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.other_case_identifiers
    ADD CONSTRAINT other_case_identifiers_pkey PRIMARY KEY (id);


--
-- Name: parent_information parent_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_information
    ADD CONSTRAINT parent_information_pkey PRIMARY KEY (id);


--
-- Name: parent_medical_history parent_medical_history_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_medical_history
    ADD CONSTRAINT parent_medical_history_pkey PRIMARY KEY (id);


--
-- Name: parent_past_drug_history parent_past_drug_history_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_past_drug_history
    ADD CONSTRAINT parent_past_drug_history_pkey PRIMARY KEY (id);


--
-- Name: past_drug_history past_drug_history_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.past_drug_history
    ADD CONSTRAINT past_drug_history_pkey PRIMARY KEY (id);


--
-- Name: patient_death_information patient_death_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_death_information
    ADD CONSTRAINT patient_death_information_pkey PRIMARY KEY (id);


--
-- Name: patient_identifiers patient_identifiers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_identifiers
    ADD CONSTRAINT patient_identifiers_pkey PRIMARY KEY (id);


--
-- Name: patient_information patient_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_information
    ADD CONSTRAINT patient_information_pkey PRIMARY KEY (id);


--
-- Name: permission_profiles permission_profiles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.permission_profiles
    ADD CONSTRAINT permission_profiles_pkey PRIMARY KEY (id);


--
-- Name: primary_sources primary_sources_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.primary_sources
    ADD CONSTRAINT primary_sources_pkey PRIMARY KEY (id);


--
-- Name: principal_authorization_state principal_authorization_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.principal_authorization_state
    ADD CONSTRAINT principal_authorization_state_pkey PRIMARY KEY (user_id, organization_id);


--
-- Name: product_presave_active_substances product_presave_active_substances_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presave_active_substances
    ADD CONSTRAINT product_presave_active_substances_pkey PRIMARY KEY (id);


--
-- Name: product_presave_active_substances product_presave_active_substances_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presave_active_substances
    ADD CONSTRAINT product_presave_active_substances_sequence_unique UNIQUE (product_presave_id, sequence_number);


--
-- Name: product_presaves product_presaves_id_organization_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presaves
    ADD CONSTRAINT product_presaves_id_organization_unique UNIQUE (id, organization_id);


--
-- Name: product_presaves product_presaves_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presaves
    ADD CONSTRAINT product_presaves_pkey PRIMARY KEY (id);


--
-- Name: rbac_policy_state rbac_policy_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rbac_policy_state
    ADD CONSTRAINT rbac_policy_state_pkey PRIMARY KEY (singleton);


--
-- Name: reactions reactions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reactions
    ADD CONSTRAINT reactions_pkey PRIMARY KEY (id);


--
-- Name: receiver_presave_consignees receiver_presave_consignees_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_consignees
    ADD CONSTRAINT receiver_presave_consignees_pkey PRIMARY KEY (id);


--
-- Name: receiver_presave_consignees receiver_presave_consignees_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_consignees
    ADD CONSTRAINT receiver_presave_consignees_sequence_unique UNIQUE (receiver_presave_id, sequence_number);


--
-- Name: receiver_presave_routes receiver_presave_routes_authority_receiver_label_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_routes
    ADD CONSTRAINT receiver_presave_routes_authority_receiver_label_unique UNIQUE (receiver_presave_id, authority, receiver_label);


--
-- Name: receiver_presave_routes receiver_presave_routes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_routes
    ADD CONSTRAINT receiver_presave_routes_pkey PRIMARY KEY (id);


--
-- Name: receiver_presave_routes receiver_presave_routes_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_routes
    ADD CONSTRAINT receiver_presave_routes_sequence_unique UNIQUE (receiver_presave_id, sequence_number);


--
-- Name: receiver_presaves receiver_presaves_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presaves
    ADD CONSTRAINT receiver_presaves_pkey PRIMARY KEY (id);


--
-- Name: relatedness_assessments relatedness_assessments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.relatedness_assessments
    ADD CONSTRAINT relatedness_assessments_pkey PRIMARY KEY (id);


--
-- Name: reported_causes_of_death reported_causes_of_death_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reported_causes_of_death
    ADD CONSTRAINT reported_causes_of_death_pkey PRIMARY KEY (id);


--
-- Name: reporter_presaves reporter_presaves_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reporter_presaves
    ADD CONSTRAINT reporter_presaves_pkey PRIMARY KEY (id);


--
-- Name: role_grants role_grants_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_grants
    ADD CONSTRAINT role_grants_pkey PRIMARY KEY (role_id, grant_id);


--
-- Name: safety_report_identification safety_report_identification_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.safety_report_identification
    ADD CONSTRAINT safety_report_identification_pkey PRIMARY KEY (id);


--
-- Name: sender_diagnoses sender_diagnoses_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_diagnoses
    ADD CONSTRAINT sender_diagnoses_pkey PRIMARY KEY (id);


--
-- Name: sender_information sender_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_information
    ADD CONSTRAINT sender_information_pkey PRIMARY KEY (id);


--
-- Name: sender_presave_gateways sender_presave_gateways_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_gateways
    ADD CONSTRAINT sender_presave_gateways_pkey PRIMARY KEY (id);


--
-- Name: sender_presave_gateways sender_presave_gateways_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_gateways
    ADD CONSTRAINT sender_presave_gateways_sequence_unique UNIQUE (sender_presave_id, sequence_number);


--
-- Name: sender_presave_responsible_persons sender_presave_responsible_persons_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_responsible_persons
    ADD CONSTRAINT sender_presave_responsible_persons_pkey PRIMARY KEY (id);


--
-- Name: sender_presave_responsible_persons sender_presave_responsible_persons_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_responsible_persons
    ADD CONSTRAINT sender_presave_responsible_persons_sequence_unique UNIQUE (sender_presave_id, sequence_number);


--
-- Name: sender_presaves sender_presaves_id_organization_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presaves
    ADD CONSTRAINT sender_presaves_id_organization_unique UNIQUE (id, organization_id);


--
-- Name: sender_presaves sender_presaves_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presaves
    ADD CONSTRAINT sender_presaves_pkey PRIMARY KEY (id);


--
-- Name: source_documents source_documents_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.source_documents
    ADD CONSTRAINT source_documents_pkey PRIMARY KEY (id);


--
-- Name: study_fda_cross_reported_inds study_fda_cross_reported_inds_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_fda_cross_reported_inds
    ADD CONSTRAINT study_fda_cross_reported_inds_pkey PRIMARY KEY (id);


--
-- Name: study_information study_information_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_information
    ADD CONSTRAINT study_information_pkey PRIMARY KEY (id);


--
-- Name: study_presave_fda_cross_reported_ind_numbers study_presave_fda_cross_reported_ind_numbers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_fda_cross_reported_ind_numbers
    ADD CONSTRAINT study_presave_fda_cross_reported_ind_numbers_pkey PRIMARY KEY (id);


--
-- Name: study_presave_fda_cross_reported_ind_numbers study_presave_fda_cross_reported_ind_numbers_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_fda_cross_reported_ind_numbers
    ADD CONSTRAINT study_presave_fda_cross_reported_ind_numbers_sequence_unique UNIQUE (study_presave_id, sequence_number);


--
-- Name: study_presave_products study_presave_products_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_products
    ADD CONSTRAINT study_presave_products_pkey PRIMARY KEY (id);


--
-- Name: study_presave_products study_presave_products_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_products
    ADD CONSTRAINT study_presave_products_sequence_unique UNIQUE (study_presave_id, sequence_number);


--
-- Name: study_presave_registration_numbers study_presave_registration_numbers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_registration_numbers
    ADD CONSTRAINT study_presave_registration_numbers_pkey PRIMARY KEY (id);


--
-- Name: study_presave_registration_numbers study_presave_registration_numbers_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_registration_numbers
    ADD CONSTRAINT study_presave_registration_numbers_sequence_unique UNIQUE (study_presave_id, sequence_number);


--
-- Name: study_presave_reporters study_presave_reporters_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_reporters
    ADD CONSTRAINT study_presave_reporters_pkey PRIMARY KEY (id);


--
-- Name: study_presave_reporters study_presave_reporters_sequence_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_reporters
    ADD CONSTRAINT study_presave_reporters_sequence_unique UNIQUE (study_presave_id, sequence_number);


--
-- Name: study_presaves study_presaves_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presaves
    ADD CONSTRAINT study_presaves_pkey PRIMARY KEY (id);


--
-- Name: study_registration_numbers study_registration_numbers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_registration_numbers
    ADD CONSTRAINT study_registration_numbers_pkey PRIMARY KEY (id);


--
-- Name: submission_acks submission_acks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_acks
    ADD CONSTRAINT submission_acks_pkey PRIMARY KEY (id);


--
-- Name: submission_dispatch_state submission_dispatch_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_dispatch_state
    ADD CONSTRAINT submission_dispatch_state_pkey PRIMARY KEY (submission_id);


--
-- Name: submission_events submission_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_events
    ADD CONSTRAINT submission_events_pkey PRIMARY KEY (id);


--
-- Name: submission_idempotency submission_idempotency_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_idempotency
    ADD CONSTRAINT submission_idempotency_pkey PRIMARY KEY (id);


--
-- Name: submission_idempotency submission_idempotency_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_idempotency
    ADD CONSTRAINT submission_idempotency_unique UNIQUE (case_id, authority, idempotency_key);


--
-- Name: terminology_releases terminology_release_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.terminology_releases
    ADD CONSTRAINT terminology_release_unique UNIQUE (dictionary, version, language);


--
-- Name: terminology_releases terminology_releases_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.terminology_releases
    ADD CONSTRAINT terminology_releases_pkey PRIMARY KEY (id);


--
-- Name: test_results test_results_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.test_results
    ADD CONSTRAINT test_results_pkey PRIMARY KEY (id);


--
-- Name: ucum_units ucum_units_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ucum_units
    ADD CONSTRAINT ucum_units_pkey PRIMARY KEY (id);


--
-- Name: case_versions unique_case_version; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_versions
    ADD CONSTRAINT unique_case_version UNIQUE (case_id, version);


--
-- Name: e2b_code_lists unique_code_per_list; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e2b_code_lists
    ADD CONSTRAINT unique_code_per_list UNIQUE (list_name, code);


--
-- Name: patient_death_information unique_death_per_patient; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_death_information
    ADD CONSTRAINT unique_death_per_patient UNIQUE (patient_id);


--
-- Name: drug_reaction_assessments unique_drug_reaction_assessment; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_reaction_assessments
    ADD CONSTRAINT unique_drug_reaction_assessment UNIQUE (drug_id, reaction_id);


--
-- Name: e2b_code_lists unique_e2b_code_lists_audit_id; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e2b_code_lists
    ADD CONSTRAINT unique_e2b_code_lists_audit_id UNIQUE (audit_id);


--
-- Name: fda_hierarchical_code_lists unique_fda_hierarchical_code; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_hierarchical_code_lists
    ADD CONSTRAINT unique_fda_hierarchical_code UNIQUE (list_name, ncit_concept_code);


--
-- Name: fda_hierarchical_code_lists unique_fda_hierarchical_code_audit_id; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_hierarchical_code_lists
    ADD CONSTRAINT unique_fda_hierarchical_code_audit_id UNIQUE (audit_id);


--
-- Name: safety_report_identification unique_identification_per_case; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.safety_report_identification
    ADD CONSTRAINT unique_identification_per_case UNIQUE (case_id);


--
-- Name: iso_countries unique_iso_countries_audit_id; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.iso_countries
    ADD CONSTRAINT unique_iso_countries_audit_id UNIQUE (audit_id);


--
-- Name: meddra_terms unique_meddra_audit_id; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.meddra_terms
    ADD CONSTRAINT unique_meddra_audit_id UNIQUE (audit_id);


--
-- Name: meddra_terms unique_meddra_code_version; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.meddra_terms
    ADD CONSTRAINT unique_meddra_code_version UNIQUE (code, version, language);


--
-- Name: narrative_information unique_narrative_per_case; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_information
    ADD CONSTRAINT unique_narrative_per_case UNIQUE (case_id);


--
-- Name: patient_information unique_patient_per_case; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_information
    ADD CONSTRAINT unique_patient_per_case UNIQUE (case_id);


--
-- Name: sender_information unique_sender_per_case; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_information
    ADD CONSTRAINT unique_sender_per_case UNIQUE (case_id);


--
-- Name: source_documents unique_source_document_sequence; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.source_documents
    ADD CONSTRAINT unique_source_document_sequence UNIQUE (case_id, sequence_number);


--
-- Name: study_information unique_study_per_case; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_information
    ADD CONSTRAINT unique_study_per_case UNIQUE (case_id);


--
-- Name: ucum_units unique_ucum_code; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ucum_units
    ADD CONSTRAINT unique_ucum_code UNIQUE (code);


--
-- Name: whodrug_products unique_whodrug_audit_id; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.whodrug_products
    ADD CONSTRAINT unique_whodrug_audit_id UNIQUE (audit_id);


--
-- Name: whodrug_products unique_whodrug_code_version; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.whodrug_products
    ADD CONSTRAINT unique_whodrug_code_version UNIQUE (code, version, language);


--
-- Name: user_organization_memberships user_organization_memberships_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_organization_memberships
    ADD CONSTRAINT user_organization_memberships_pkey PRIMARY KEY (user_id, organization_id);


--
-- Name: user_role_assignments user_role_assignments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_role_assignments
    ADD CONSTRAINT user_role_assignments_pkey PRIMARY KEY (user_id, organization_id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: whodrug_products whodrug_products_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.whodrug_products
    ADD CONSTRAINT whodrug_products_pkey PRIMARY KEY (id);


--
-- Name: xml_export_history xml_export_history_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.xml_export_history
    ADD CONSTRAINT xml_export_history_pkey PRIMARY KEY (id);


--
-- Name: xml_import_history xml_import_history_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.xml_import_history
    ADD CONSTRAINT xml_import_history_pkey PRIMARY KEY (id);


--
-- Name: authorization_migration_rejections_unresolved; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX authorization_migration_rejections_unresolved ON public.authorization_migration_rejections USING btree (user_id, organization_id, legacy_role, reason) NULLS NOT DISTINCT WHERE (NOT resolved);


--
-- Name: authorization_roles_builtin_identity_kind; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX authorization_roles_builtin_identity_kind ON public.authorization_roles USING btree (identity_kind) WHERE built_in;


--
-- Name: authorization_roles_builtin_stable_key; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX authorization_roles_builtin_stable_key ON public.authorization_roles USING btree (stable_key) WHERE built_in;


--
-- Name: authorization_roles_custom_org_name; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX authorization_roles_custom_org_name ON public.authorization_roles USING btree (organization_id, lower(btrim(name))) WHERE ((NOT built_in) AND active);


--
-- Name: drug_information_case_source_xml_id; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX drug_information_case_source_xml_id ON public.drug_information USING btree (case_id, source_xml_id) WHERE (source_xml_id IS NOT NULL);


--
-- Name: idx_active_substances_drug; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_active_substances_drug ON public.drug_active_substances USING btree (drug_id);


--
-- Name: idx_app_settings_org_key; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_app_settings_org_key ON public.app_settings USING btree (organization_id, key);


--
-- Name: idx_audit_logs_changed_fields; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_changed_fields ON public.audit_logs USING gin (changed_fields);


--
-- Name: idx_audit_logs_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_created_at ON public.audit_logs USING btree (created_at);


--
-- Name: idx_audit_logs_entry_hash; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_audit_logs_entry_hash ON public.audit_logs USING btree (entry_hash);


--
-- Name: idx_audit_logs_esignature; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_esignature ON public.audit_logs USING btree (e_signature_id);


--
-- Name: idx_audit_logs_org_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_org_created_at ON public.audit_logs USING btree (organization_id, created_at DESC);


--
-- Name: idx_audit_logs_org_table_record_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_org_table_record_created_at ON public.audit_logs USING btree (organization_id, table_name, record_id, created_at DESC);


--
-- Name: idx_audit_logs_org_user_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_org_user_created_at ON public.audit_logs USING btree (organization_id, user_id, created_at DESC);


--
-- Name: idx_audit_logs_parent_record_ids; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_parent_record_ids ON public.audit_logs USING gin (parent_record_ids);


--
-- Name: idx_audit_logs_prev_hash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_prev_hash ON public.audit_logs USING btree (prev_hash);


--
-- Name: idx_audit_logs_table_record; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_table_record ON public.audit_logs USING btree (table_name, record_id);


--
-- Name: idx_audit_logs_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_user ON public.audit_logs USING btree (user_id);


--
-- Name: idx_authorization_audit_org_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_authorization_audit_org_created ON public.authorization_audit_events USING btree (organization_id, created_at DESC);


--
-- Name: idx_authorization_audit_request; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_authorization_audit_request ON public.authorization_audit_events USING btree (request_id);


--
-- Name: idx_autopsy_causes_of_death_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_autopsy_causes_of_death_active_sequence_unique ON public.autopsy_causes_of_death USING btree (death_info_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_case_submissions_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_case_submissions_case ON public.case_submissions USING btree (case_id, submitted_at DESC);


--
-- Name: idx_case_submissions_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_case_submissions_status ON public.case_submissions USING btree (status, updated_at DESC);


--
-- Name: idx_case_summary; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_case_summary ON public.case_summary_information USING btree (narrative_id);


--
-- Name: idx_case_summary_information_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_case_summary_information_active_sequence_unique ON public.case_summary_information USING btree (narrative_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_case_validation_summaries_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_case_validation_summaries_case ON public.case_validation_summaries USING btree (case_id);


--
-- Name: idx_case_validation_summaries_page; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_case_validation_summaries_page ON public.case_validation_summaries USING btree (case_id, page_id, stale);


--
-- Name: idx_case_versions_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_case_versions_case ON public.case_versions USING btree (case_id);


--
-- Name: idx_case_workflow_events_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_case_workflow_events_case ON public.case_workflow_events USING btree (case_id, created_at DESC);


--
-- Name: idx_cases_created_by; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_cases_created_by ON public.cases USING btree (created_by);


--
-- Name: idx_cases_organization; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_cases_organization ON public.cases USING btree (organization_id);


--
-- Name: idx_cases_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_cases_status ON public.cases USING btree (status);


--
-- Name: idx_cases_workflow_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_cases_workflow_status ON public.cases USING btree (workflow_status);


--
-- Name: idx_code_lists_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_code_lists_name ON public.e2b_code_lists USING btree (list_name);


--
-- Name: idx_controlled_terminology_active_lookup; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_controlled_terminology_active_lookup ON public.controlled_terminology_terms USING btree (dictionary, scope, code) WHERE (active = true);


--
-- Name: idx_dashboard_notices_org_order; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_dashboard_notices_org_order ON public.dashboard_notices USING btree (organization_id, sort_order, created_at);


--
-- Name: idx_death_info_patient; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_death_info_patient ON public.patient_death_information USING btree (patient_id);


--
-- Name: idx_documents_held_by_sender_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_documents_held_by_sender_active_sequence_unique ON public.documents_held_by_sender USING btree (case_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_documents_held_by_sender_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_documents_held_by_sender_case ON public.documents_held_by_sender USING btree (case_id);


--
-- Name: idx_dosage_info_drug; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_dosage_info_drug ON public.dosage_information USING btree (drug_id);


--
-- Name: idx_dosage_information_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_dosage_information_active_sequence_unique ON public.dosage_information USING btree (drug_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_drug_active_substances_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_drug_active_substances_active_sequence_unique ON public.drug_active_substances USING btree (drug_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_drug_device_characteristics_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_drug_device_characteristics_active_sequence_unique ON public.drug_device_characteristics USING btree (drug_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_drug_device_characteristics_drug; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_drug_device_characteristics_drug ON public.drug_device_characteristics USING btree (drug_id);


--
-- Name: idx_drug_indications_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_drug_indications_active_sequence_unique ON public.drug_indications USING btree (drug_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_drug_indications_drug; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_drug_indications_drug ON public.drug_indications USING btree (drug_id);


--
-- Name: idx_drug_info_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_drug_info_case ON public.drug_information USING btree (case_id);


--
-- Name: idx_drug_info_mfds_mpid; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_drug_info_mfds_mpid ON public.drug_information USING btree (mfds_mpid);


--
-- Name: idx_drug_info_source_presave; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_drug_info_source_presave ON public.drug_information USING btree (source_product_presave_id);


--
-- Name: idx_drug_information_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_drug_information_active_sequence_unique ON public.drug_information USING btree (case_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_drug_reaction_assessments_drug; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_drug_reaction_assessments_drug ON public.drug_reaction_assessments USING btree (drug_id);


--
-- Name: idx_drug_reaction_assessments_reaction; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_drug_reaction_assessments_reaction ON public.drug_reaction_assessments USING btree (reaction_id);


--
-- Name: idx_e_signatures_action; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_e_signatures_action ON public.e_signatures USING btree (action, signed_at DESC);


--
-- Name: idx_e_signatures_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_e_signatures_case ON public.e_signatures USING btree (case_id, signed_at DESC);


--
-- Name: idx_e_signatures_signer; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_e_signatures_signer ON public.e_signatures USING btree (signer_user_id, signed_at DESC);


--
-- Name: idx_fda_device_codes_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_fda_device_codes_active_sequence_unique ON public.fda_device_codes USING btree (device_id, element, sequence_number) WHERE (deleted = false);


--
-- Name: idx_fda_device_codes_device; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fda_device_codes_device ON public.fda_device_codes USING btree (device_id);


--
-- Name: idx_fda_device_information_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_fda_device_information_active_sequence_unique ON public.fda_device_information USING btree (drug_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_fda_device_information_drug; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fda_device_information_drug ON public.fda_device_information USING btree (drug_id);


--
-- Name: idx_fda_hierarchical_code_level1; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fda_hierarchical_code_level1 ON public.fda_hierarchical_code_lists USING btree (level1_term);


--
-- Name: idx_fda_hierarchical_code_level2; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fda_hierarchical_code_level2 ON public.fda_hierarchical_code_lists USING btree (level2_term);


--
-- Name: idx_fda_hierarchical_code_level3; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fda_hierarchical_code_level3 ON public.fda_hierarchical_code_lists USING btree (level3_term);


--
-- Name: idx_fda_hierarchical_code_list_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fda_hierarchical_code_list_name ON public.fda_hierarchical_code_lists USING btree (list_name);


--
-- Name: idx_linked_report_numbers_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_linked_report_numbers_active_sequence_unique ON public.linked_report_numbers USING btree (case_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_linked_report_numbers_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_linked_report_numbers_case ON public.linked_report_numbers USING btree (case_id);


--
-- Name: idx_literature_references_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_literature_references_active_sequence_unique ON public.literature_references USING btree (case_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_literature_refs_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_literature_refs_case ON public.literature_references USING btree (case_id);


--
-- Name: idx_med_history_patient; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_med_history_patient ON public.medical_history_episodes USING btree (patient_id);


--
-- Name: idx_meddra_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_meddra_code ON public.meddra_terms USING btree (code);


--
-- Name: idx_meddra_level; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_meddra_level ON public.meddra_terms USING btree (level);


--
-- Name: idx_meddra_release; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_meddra_release ON public.meddra_terms USING btree (version, language);


--
-- Name: idx_meddra_term; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_meddra_term ON public.meddra_terms USING gin (to_tsvector('english'::regconfig, (term)::text));


--
-- Name: idx_meddra_version; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_meddra_version ON public.meddra_terms USING btree (version);


--
-- Name: idx_medical_history_episodes_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_medical_history_episodes_active_sequence_unique ON public.medical_history_episodes USING btree (patient_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_mfds_product_substances_lookup; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mfds_product_substances_lookup ON public.mfds_product_substances USING btree (item_seq, version) WHERE (active = true);


--
-- Name: idx_mfds_products_active_item_seq; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mfds_products_active_item_seq ON public.mfds_products USING btree (item_seq) WHERE (active = true);


--
-- Name: idx_mfds_products_name_kr; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_mfds_products_name_kr ON public.mfds_products USING gin (to_tsvector('simple'::regconfig, product_name_kr));


--
-- Name: idx_narrative_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_narrative_case ON public.narrative_information USING btree (case_id);


--
-- Name: idx_narrative_presaves_org; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_narrative_presaves_org ON public.narrative_presaves USING btree (organization_id);


--
-- Name: idx_narrative_source_presave; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_narrative_source_presave ON public.narrative_information USING btree (source_narrative_presave_id);


--
-- Name: idx_other_case_identifiers_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_other_case_identifiers_active_sequence_unique ON public.other_case_identifiers USING btree (case_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_other_case_identifiers_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_other_case_identifiers_case ON public.other_case_identifiers USING btree (case_id);


--
-- Name: idx_parent_info_patient; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_parent_info_patient ON public.parent_information USING btree (patient_id);


--
-- Name: idx_parent_information_active_patient_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_parent_information_active_patient_unique ON public.parent_information USING btree (patient_id) WHERE (deleted = false);


--
-- Name: idx_parent_med_history_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_parent_med_history_parent ON public.parent_medical_history USING btree (parent_id);


--
-- Name: idx_parent_medical_history_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_parent_medical_history_active_sequence_unique ON public.parent_medical_history USING btree (parent_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_parent_past_drug_history_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_parent_past_drug_history_active_sequence_unique ON public.parent_past_drug_history USING btree (parent_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_parent_past_drug_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_parent_past_drug_parent ON public.parent_past_drug_history USING btree (parent_id);


--
-- Name: idx_past_drug_history_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_past_drug_history_active_sequence_unique ON public.past_drug_history USING btree (patient_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_past_drug_patient; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_past_drug_patient ON public.past_drug_history USING btree (patient_id);


--
-- Name: idx_patient_identifiers_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_patient_identifiers_active_sequence_unique ON public.patient_identifiers USING btree (patient_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_patient_identifiers_patient; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_patient_identifiers_patient ON public.patient_identifiers USING btree (patient_id);


--
-- Name: idx_patient_info_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_patient_info_case ON public.patient_information USING btree (case_id);


--
-- Name: idx_permission_profiles_org; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_permission_profiles_org ON public.permission_profiles USING btree (organization_id);


--
-- Name: idx_permission_profiles_org_name_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_permission_profiles_org_name_unique ON public.permission_profiles USING btree (organization_id, lower(btrim((name)::text)));


--
-- Name: idx_primary_sources_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_primary_sources_active_sequence_unique ON public.primary_sources USING btree (case_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_primary_sources_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_primary_sources_case ON public.primary_sources USING btree (case_id);


--
-- Name: idx_primary_sources_source_presave; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_primary_sources_source_presave ON public.primary_sources USING btree (source_reporter_presave_id);


--
-- Name: idx_product_presave_active_substances_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_product_presave_active_substances_parent ON public.product_presave_active_substances USING btree (product_presave_id);


--
-- Name: idx_product_presaves_org; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_product_presaves_org ON public.product_presaves USING btree (organization_id);


--
-- Name: idx_product_presaves_receiver; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_product_presaves_receiver ON public.product_presaves USING btree (receiver_presave_id) WHERE (receiver_presave_id IS NOT NULL);


--
-- Name: idx_product_presaves_sender; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_product_presaves_sender ON public.product_presaves USING btree (sender_presave_id);


--
-- Name: idx_reactions_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_reactions_active_sequence_unique ON public.reactions USING btree (case_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_reactions_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reactions_case ON public.reactions USING btree (case_id);


--
-- Name: idx_reactions_meddra; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reactions_meddra ON public.reactions USING btree (reaction_meddra_code);


--
-- Name: idx_receiver_presave_consignees_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receiver_presave_consignees_parent ON public.receiver_presave_consignees USING btree (receiver_presave_id);


--
-- Name: idx_receiver_presave_routes_authority; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receiver_presave_routes_authority ON public.receiver_presave_routes USING btree (authority);


--
-- Name: idx_receiver_presave_routes_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receiver_presave_routes_parent ON public.receiver_presave_routes USING btree (receiver_presave_id);


--
-- Name: idx_receiver_presaves_org; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receiver_presaves_org ON public.receiver_presaves USING btree (organization_id);


--
-- Name: idx_relatedness_assessments_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_relatedness_assessments_active_sequence_unique ON public.relatedness_assessments USING btree (drug_reaction_assessment_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_relatedness_assessments_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_relatedness_assessments_parent ON public.relatedness_assessments USING btree (drug_reaction_assessment_id);


--
-- Name: idx_reported_causes_of_death_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_reported_causes_of_death_active_sequence_unique ON public.reported_causes_of_death USING btree (death_info_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_reporter_presaves_org; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_reporter_presaves_org ON public.reporter_presaves USING btree (organization_id);


--
-- Name: idx_safety_report_id_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_safety_report_id_case ON public.safety_report_identification USING btree (case_id);


--
-- Name: idx_safety_report_id_worldwide; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_safety_report_id_worldwide ON public.safety_report_identification USING btree (worldwide_unique_id);


--
-- Name: idx_safety_report_identification_report_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_safety_report_identification_report_id ON public.safety_report_identification USING btree (safety_report_id);


--
-- Name: idx_sender_diagnoses; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sender_diagnoses ON public.sender_diagnoses USING btree (narrative_id);


--
-- Name: idx_sender_diagnoses_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_sender_diagnoses_active_sequence_unique ON public.sender_diagnoses USING btree (narrative_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_sender_info_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sender_info_case ON public.sender_information USING btree (case_id);


--
-- Name: idx_sender_info_source_presave; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sender_info_source_presave ON public.sender_information USING btree (source_sender_presave_id);


--
-- Name: idx_sender_presave_gateways_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sender_presave_gateways_parent ON public.sender_presave_gateways USING btree (sender_presave_id);


--
-- Name: idx_sender_presave_responsible_persons_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sender_presave_responsible_persons_parent ON public.sender_presave_responsible_persons USING btree (sender_presave_id);


--
-- Name: idx_sender_presaves_org; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sender_presaves_org ON public.sender_presaves USING btree (organization_id);


--
-- Name: idx_source_documents_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_source_documents_case ON public.source_documents USING btree (case_id);


--
-- Name: idx_study_fda_cross_reported_inds; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_fda_cross_reported_inds ON public.study_fda_cross_reported_inds USING btree (study_information_id);


--
-- Name: idx_study_fda_cross_reported_inds_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_study_fda_cross_reported_inds_active_sequence_unique ON public.study_fda_cross_reported_inds USING btree (study_information_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_study_info_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_info_case ON public.study_information USING btree (case_id);


--
-- Name: idx_study_info_source_presave; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_info_source_presave ON public.study_information USING btree (source_study_presave_id);


--
-- Name: idx_study_presave_fda_cross_reported_ind_numbers_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_presave_fda_cross_reported_ind_numbers_parent ON public.study_presave_fda_cross_reported_ind_numbers USING btree (study_presave_id);


--
-- Name: idx_study_presave_products_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_presave_products_parent ON public.study_presave_products USING btree (study_presave_id);


--
-- Name: idx_study_presave_registration_numbers_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_presave_registration_numbers_parent ON public.study_presave_registration_numbers USING btree (study_presave_id);


--
-- Name: idx_study_presave_reporters_parent; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_presave_reporters_parent ON public.study_presave_reporters USING btree (study_presave_id);


--
-- Name: idx_study_presaves_org; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_presaves_org ON public.study_presaves USING btree (organization_id);


--
-- Name: idx_study_presaves_product; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_presaves_product ON public.study_presaves USING btree (product_presave_id);


--
-- Name: idx_study_reg_nums; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_study_reg_nums ON public.study_registration_numbers USING btree (study_information_id);


--
-- Name: idx_study_registration_numbers_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_study_registration_numbers_active_sequence_unique ON public.study_registration_numbers USING btree (study_information_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_submission_acks_submission; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_submission_acks_submission ON public.submission_acks USING btree (submission_id, ack_level, received_at DESC);


--
-- Name: idx_submission_acks_unique_event; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_submission_acks_unique_event ON public.submission_acks USING btree (submission_id, ack_level, success, COALESCE(ack_code, ''::character varying), received_at);


--
-- Name: idx_submission_dispatch_retry; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_submission_dispatch_retry ON public.submission_dispatch_state USING btree (next_retry_at) WHERE (next_retry_at IS NOT NULL);


--
-- Name: idx_submission_dispatch_terminal; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_submission_dispatch_terminal ON public.submission_dispatch_state USING btree (terminal_at);


--
-- Name: idx_submission_events_submission; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_submission_events_submission ON public.submission_events USING btree (submission_id, created_at DESC);


--
-- Name: idx_submission_events_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_submission_events_type ON public.submission_events USING btree (event_type, created_at DESC);


--
-- Name: idx_submission_idempotency_submission; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_submission_idempotency_submission ON public.submission_idempotency USING btree (submission_id);


--
-- Name: idx_terminology_releases_lookup; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_terminology_releases_lookup ON public.terminology_releases USING btree (dictionary, language, status, activated_at DESC);


--
-- Name: idx_test_results_active_sequence_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_test_results_active_sequence_unique ON public.test_results USING btree (case_id, sequence_number) WHERE (deleted = false);


--
-- Name: idx_test_results_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_test_results_case ON public.test_results USING btree (case_id);


--
-- Name: idx_user_organization_memberships_org; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_organization_memberships_org ON public.user_organization_memberships USING btree (organization_id);


--
-- Name: idx_users_email; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_users_email ON public.users USING btree (email);


--
-- Name: idx_users_organization; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_users_organization ON public.users USING btree (organization_id);


--
-- Name: idx_whodrug_atc; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_whodrug_atc ON public.whodrug_products USING btree (atc_code);


--
-- Name: idx_whodrug_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_whodrug_code ON public.whodrug_products USING btree (code);


--
-- Name: idx_whodrug_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_whodrug_name ON public.whodrug_products USING gin (to_tsvector('english'::regconfig, drug_name));


--
-- Name: idx_whodrug_release; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_whodrug_release ON public.whodrug_products USING btree (version, language);


--
-- Name: idx_whodrug_search_trgm; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_whodrug_search_trgm ON public.whodrug_products USING gin (drug_name public.gin_trgm_ops);


--
-- Name: idx_xml_export_history_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_xml_export_history_case ON public.xml_export_history USING btree (case_id, exported_at DESC);


--
-- Name: idx_xml_export_history_exported_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_xml_export_history_exported_at ON public.xml_export_history USING btree (exported_at DESC);


--
-- Name: idx_xml_export_history_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_xml_export_history_user ON public.xml_export_history USING btree (exported_by, exported_at DESC);


--
-- Name: idx_xml_import_history_case; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_xml_import_history_case ON public.xml_import_history USING btree (case_id, uploaded_at DESC);


--
-- Name: idx_xml_import_history_uploaded_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_xml_import_history_uploaded_at ON public.xml_import_history USING btree (uploaded_at DESC);


--
-- Name: idx_xml_import_history_user; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_xml_import_history_user ON public.xml_import_history USING btree (uploaded_by, uploaded_at DESC);


--
-- Name: meddra_one_active_release; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX meddra_one_active_release ON public.terminology_releases USING btree (dictionary, language) WHERE (((dictionary)::text = 'meddra'::text) AND ((status)::text = 'active'::text));


--
-- Name: organization_as2_profiles_one_active; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX organization_as2_profiles_one_active ON public.organization_as2_profiles USING btree (organization_id, authority) WHERE active;


--
-- Name: reactions_case_source_xml_id; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX reactions_case_source_xml_id ON public.reactions USING btree (case_id, source_xml_id) WHERE (source_xml_id IS NOT NULL);


--
-- Name: terminology_releases_audit_id_key; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX terminology_releases_audit_id_key ON public.terminology_releases USING btree (audit_id);


--
-- Name: users_email_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX users_email_unique ON public.users USING btree (lower(btrim((email)::text)));


--
-- Name: users_username_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX users_username_unique ON public.users USING btree (lower(btrim((username)::text)));


--
-- Name: whodrug_one_active_release; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX whodrug_one_active_release ON public.terminology_releases USING btree (dictionary, language) WHERE (((dictionary)::text = 'whodrug'::text) AND ((status)::text = 'active'::text));


--
-- Name: app_settings app_settings_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_settings
    ADD CONSTRAINT app_settings_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: app_settings app_settings_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_settings
    ADD CONSTRAINT app_settings_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: audit_logs audit_logs_e_signature_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_e_signature_id_fkey FOREIGN KEY (e_signature_id) REFERENCES public.e_signatures(id);


--
-- Name: audit_logs audit_logs_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: audit_logs audit_logs_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: authorization_grant_role_classes authorization_grant_role_classes_grant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_grant_role_classes
    ADD CONSTRAINT authorization_grant_role_classes_grant_id_fkey FOREIGN KEY (grant_id) REFERENCES public.authorization_grant_catalog(grant_id) ON DELETE CASCADE;


--
-- Name: authorization_roles authorization_roles_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.authorization_roles
    ADD CONSTRAINT authorization_roles_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: autopsy_causes_of_death autopsy_causes_of_death_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.autopsy_causes_of_death
    ADD CONSTRAINT autopsy_causes_of_death_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: autopsy_causes_of_death autopsy_causes_of_death_death_info_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.autopsy_causes_of_death
    ADD CONSTRAINT autopsy_causes_of_death_death_info_id_fkey FOREIGN KEY (death_info_id) REFERENCES public.patient_death_information(id) ON DELETE CASCADE;


--
-- Name: autopsy_causes_of_death autopsy_causes_of_death_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.autopsy_causes_of_death
    ADD CONSTRAINT autopsy_causes_of_death_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: case_e2b_field_notations case_e2b_field_notations_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_e2b_field_notations
    ADD CONSTRAINT case_e2b_field_notations_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: case_e2b_field_notations case_e2b_field_notations_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_e2b_field_notations
    ADD CONSTRAINT case_e2b_field_notations_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id);


--
-- Name: case_e2b_field_notations case_e2b_field_notations_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_e2b_field_notations
    ADD CONSTRAINT case_e2b_field_notations_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id);


--
-- Name: case_field_notations case_field_notations_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_field_notations
    ADD CONSTRAINT case_field_notations_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: case_field_notations case_field_notations_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_field_notations
    ADD CONSTRAINT case_field_notations_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id);


--
-- Name: case_field_notations case_field_notations_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_field_notations
    ADD CONSTRAINT case_field_notations_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id);


--
-- Name: case_submissions case_submissions_as2_profile_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_submissions
    ADD CONSTRAINT case_submissions_as2_profile_id_fkey FOREIGN KEY (as2_profile_id) REFERENCES public.organization_as2_profiles(id) ON DELETE RESTRICT;


--
-- Name: case_submissions case_submissions_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_submissions
    ADD CONSTRAINT case_submissions_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: case_submissions case_submissions_submitted_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_submissions
    ADD CONSTRAINT case_submissions_submitted_by_fkey FOREIGN KEY (submitted_by) REFERENCES public.users(id);


--
-- Name: case_submissions case_submissions_submitting_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_submissions
    ADD CONSTRAINT case_submissions_submitting_organization_id_fkey FOREIGN KEY (submitting_organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: case_summary_information case_summary_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_summary_information
    ADD CONSTRAINT case_summary_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: case_summary_information case_summary_information_narrative_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_summary_information
    ADD CONSTRAINT case_summary_information_narrative_id_fkey FOREIGN KEY (narrative_id) REFERENCES public.narrative_information(id) ON DELETE CASCADE;


--
-- Name: case_summary_information case_summary_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_summary_information
    ADD CONSTRAINT case_summary_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: case_validation_summaries case_validation_summaries_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_validation_summaries
    ADD CONSTRAINT case_validation_summaries_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: case_versions case_versions_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_versions
    ADD CONSTRAINT case_versions_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: case_versions case_versions_changed_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_versions
    ADD CONSTRAINT case_versions_changed_by_fkey FOREIGN KEY (changed_by) REFERENCES public.users(id);


--
-- Name: case_workflow_events case_workflow_events_acted_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_workflow_events
    ADD CONSTRAINT case_workflow_events_acted_by_fkey FOREIGN KEY (acted_by) REFERENCES public.users(id);


--
-- Name: case_workflow_events case_workflow_events_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_workflow_events
    ADD CONSTRAINT case_workflow_events_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: case_workflow_events case_workflow_events_from_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_workflow_events
    ADD CONSTRAINT case_workflow_events_from_user_id_fkey FOREIGN KEY (from_user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: case_workflow_events case_workflow_events_target_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.case_workflow_events
    ADD CONSTRAINT case_workflow_events_target_user_id_fkey FOREIGN KEY (target_user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: cases cases_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cases
    ADD CONSTRAINT cases_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id);


--
-- Name: cases cases_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cases
    ADD CONSTRAINT cases_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: cases cases_submitted_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cases
    ADD CONSTRAINT cases_submitted_by_fkey FOREIGN KEY (submitted_by) REFERENCES public.users(id);


--
-- Name: cases cases_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cases
    ADD CONSTRAINT cases_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id);


--
-- Name: cases cases_workflow_assigned_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cases
    ADD CONSTRAINT cases_workflow_assigned_user_id_fkey FOREIGN KEY (workflow_assigned_user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: dashboard_notices dashboard_notices_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dashboard_notices
    ADD CONSTRAINT dashboard_notices_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: dashboard_notices dashboard_notices_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dashboard_notices
    ADD CONSTRAINT dashboard_notices_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: documents_held_by_sender documents_held_by_sender_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.documents_held_by_sender
    ADD CONSTRAINT documents_held_by_sender_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: documents_held_by_sender documents_held_by_sender_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.documents_held_by_sender
    ADD CONSTRAINT documents_held_by_sender_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: documents_held_by_sender documents_held_by_sender_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.documents_held_by_sender
    ADD CONSTRAINT documents_held_by_sender_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: dosage_information dosage_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dosage_information
    ADD CONSTRAINT dosage_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: dosage_information dosage_information_drug_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dosage_information
    ADD CONSTRAINT dosage_information_drug_id_fkey FOREIGN KEY (drug_id) REFERENCES public.drug_information(id) ON DELETE CASCADE;


--
-- Name: dosage_information dosage_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dosage_information
    ADD CONSTRAINT dosage_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_active_substances drug_active_substances_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_active_substances
    ADD CONSTRAINT drug_active_substances_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_active_substances drug_active_substances_drug_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_active_substances
    ADD CONSTRAINT drug_active_substances_drug_id_fkey FOREIGN KEY (drug_id) REFERENCES public.drug_information(id) ON DELETE CASCADE;


--
-- Name: drug_active_substances drug_active_substances_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_active_substances
    ADD CONSTRAINT drug_active_substances_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_device_characteristics drug_device_characteristics_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_device_characteristics
    ADD CONSTRAINT drug_device_characteristics_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_device_characteristics drug_device_characteristics_drug_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_device_characteristics
    ADD CONSTRAINT drug_device_characteristics_drug_id_fkey FOREIGN KEY (drug_id) REFERENCES public.drug_information(id) ON DELETE CASCADE;


--
-- Name: drug_device_characteristics drug_device_characteristics_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_device_characteristics
    ADD CONSTRAINT drug_device_characteristics_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_indications drug_indications_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_indications
    ADD CONSTRAINT drug_indications_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_indications drug_indications_drug_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_indications
    ADD CONSTRAINT drug_indications_drug_id_fkey FOREIGN KEY (drug_id) REFERENCES public.drug_information(id) ON DELETE CASCADE;


--
-- Name: drug_indications drug_indications_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_indications
    ADD CONSTRAINT drug_indications_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_information drug_information_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_information
    ADD CONSTRAINT drug_information_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: drug_information drug_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_information
    ADD CONSTRAINT drug_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_information drug_information_source_product_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_information
    ADD CONSTRAINT drug_information_source_product_presave_id_fkey FOREIGN KEY (source_product_presave_id) REFERENCES public.product_presaves(id) ON DELETE SET NULL;


--
-- Name: drug_information drug_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_information
    ADD CONSTRAINT drug_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_reaction_assessments drug_reaction_assessments_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_reaction_assessments
    ADD CONSTRAINT drug_reaction_assessments_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: drug_reaction_assessments drug_reaction_assessments_drug_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_reaction_assessments
    ADD CONSTRAINT drug_reaction_assessments_drug_id_fkey FOREIGN KEY (drug_id) REFERENCES public.drug_information(id) ON DELETE CASCADE;


--
-- Name: drug_reaction_assessments drug_reaction_assessments_reaction_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_reaction_assessments
    ADD CONSTRAINT drug_reaction_assessments_reaction_id_fkey FOREIGN KEY (reaction_id) REFERENCES public.reactions(id) ON DELETE CASCADE;


--
-- Name: drug_reaction_assessments drug_reaction_assessments_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.drug_reaction_assessments
    ADD CONSTRAINT drug_reaction_assessments_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: e_signatures e_signatures_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e_signatures
    ADD CONSTRAINT e_signatures_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE SET NULL;


--
-- Name: e_signatures e_signatures_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e_signatures
    ADD CONSTRAINT e_signatures_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id);


--
-- Name: e_signatures e_signatures_signer_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e_signatures
    ADD CONSTRAINT e_signatures_signer_user_id_fkey FOREIGN KEY (signer_user_id) REFERENCES public.users(id);


--
-- Name: e_signatures e_signatures_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.e_signatures
    ADD CONSTRAINT e_signatures_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id);


--
-- Name: fda_device_codes fda_device_codes_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_device_codes
    ADD CONSTRAINT fda_device_codes_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: fda_device_codes fda_device_codes_device_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_device_codes
    ADD CONSTRAINT fda_device_codes_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.fda_device_information(id) ON DELETE CASCADE;


--
-- Name: fda_device_codes fda_device_codes_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_device_codes
    ADD CONSTRAINT fda_device_codes_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: fda_device_information fda_device_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_device_information
    ADD CONSTRAINT fda_device_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: fda_device_information fda_device_information_drug_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_device_information
    ADD CONSTRAINT fda_device_information_drug_id_fkey FOREIGN KEY (drug_id) REFERENCES public.drug_information(id) ON DELETE CASCADE;


--
-- Name: fda_device_information fda_device_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fda_device_information
    ADD CONSTRAINT fda_device_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: organizations fk_organizations_created_by; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT fk_organizations_created_by FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: organizations fk_organizations_updated_by; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT fk_organizations_updated_by FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: users fk_users_created_by; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT fk_users_created_by FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: users fk_users_organization; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT fk_users_organization FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: users fk_users_updated_by; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT fk_users_updated_by FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: linked_report_numbers linked_report_numbers_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.linked_report_numbers
    ADD CONSTRAINT linked_report_numbers_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: linked_report_numbers linked_report_numbers_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.linked_report_numbers
    ADD CONSTRAINT linked_report_numbers_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: linked_report_numbers linked_report_numbers_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.linked_report_numbers
    ADD CONSTRAINT linked_report_numbers_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: literature_references literature_references_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.literature_references
    ADD CONSTRAINT literature_references_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: literature_references literature_references_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.literature_references
    ADD CONSTRAINT literature_references_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: literature_references literature_references_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.literature_references
    ADD CONSTRAINT literature_references_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: medical_history_episodes medical_history_episodes_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.medical_history_episodes
    ADD CONSTRAINT medical_history_episodes_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: medical_history_episodes medical_history_episodes_patient_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.medical_history_episodes
    ADD CONSTRAINT medical_history_episodes_patient_id_fkey FOREIGN KEY (patient_id) REFERENCES public.patient_information(id) ON DELETE CASCADE;


--
-- Name: medical_history_episodes medical_history_episodes_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.medical_history_episodes
    ADD CONSTRAINT medical_history_episodes_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: mfds_product_substances mfds_product_substances_product_fk; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mfds_product_substances
    ADD CONSTRAINT mfds_product_substances_product_fk FOREIGN KEY (item_seq, version) REFERENCES public.mfds_products(item_seq, version) ON DELETE CASCADE;


--
-- Name: narrative_information narrative_information_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_information
    ADD CONSTRAINT narrative_information_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: narrative_information narrative_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_information
    ADD CONSTRAINT narrative_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: narrative_information narrative_information_source_narrative_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_information
    ADD CONSTRAINT narrative_information_source_narrative_presave_id_fkey FOREIGN KEY (source_narrative_presave_id) REFERENCES public.narrative_presaves(id) ON DELETE SET NULL;


--
-- Name: narrative_information narrative_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_information
    ADD CONSTRAINT narrative_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: narrative_presaves narrative_presaves_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_presaves
    ADD CONSTRAINT narrative_presaves_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: narrative_presaves narrative_presaves_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_presaves
    ADD CONSTRAINT narrative_presaves_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: narrative_presaves narrative_presaves_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.narrative_presaves
    ADD CONSTRAINT narrative_presaves_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: organization_as2_profiles organization_as2_profiles_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_as2_profiles
    ADD CONSTRAINT organization_as2_profiles_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: organization_as2_profiles organization_as2_profiles_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_as2_profiles
    ADD CONSTRAINT organization_as2_profiles_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: organization_as2_profiles organization_as2_profiles_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_as2_profiles
    ADD CONSTRAINT organization_as2_profiles_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: organization_policy_state organization_policy_state_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.organization_policy_state
    ADD CONSTRAINT organization_policy_state_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: other_case_identifiers other_case_identifiers_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.other_case_identifiers
    ADD CONSTRAINT other_case_identifiers_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: other_case_identifiers other_case_identifiers_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.other_case_identifiers
    ADD CONSTRAINT other_case_identifiers_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: other_case_identifiers other_case_identifiers_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.other_case_identifiers
    ADD CONSTRAINT other_case_identifiers_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: parent_information parent_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_information
    ADD CONSTRAINT parent_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: parent_information parent_information_patient_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_information
    ADD CONSTRAINT parent_information_patient_id_fkey FOREIGN KEY (patient_id) REFERENCES public.patient_information(id) ON DELETE CASCADE;


--
-- Name: parent_information parent_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_information
    ADD CONSTRAINT parent_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: parent_medical_history parent_medical_history_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_medical_history
    ADD CONSTRAINT parent_medical_history_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: parent_medical_history parent_medical_history_parent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_medical_history
    ADD CONSTRAINT parent_medical_history_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES public.parent_information(id) ON DELETE CASCADE;


--
-- Name: parent_medical_history parent_medical_history_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_medical_history
    ADD CONSTRAINT parent_medical_history_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: parent_past_drug_history parent_past_drug_history_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_past_drug_history
    ADD CONSTRAINT parent_past_drug_history_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: parent_past_drug_history parent_past_drug_history_parent_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_past_drug_history
    ADD CONSTRAINT parent_past_drug_history_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES public.parent_information(id) ON DELETE CASCADE;


--
-- Name: parent_past_drug_history parent_past_drug_history_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.parent_past_drug_history
    ADD CONSTRAINT parent_past_drug_history_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: past_drug_history past_drug_history_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.past_drug_history
    ADD CONSTRAINT past_drug_history_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: past_drug_history past_drug_history_patient_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.past_drug_history
    ADD CONSTRAINT past_drug_history_patient_id_fkey FOREIGN KEY (patient_id) REFERENCES public.patient_information(id) ON DELETE CASCADE;


--
-- Name: past_drug_history past_drug_history_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.past_drug_history
    ADD CONSTRAINT past_drug_history_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: patient_death_information patient_death_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_death_information
    ADD CONSTRAINT patient_death_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: patient_death_information patient_death_information_patient_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_death_information
    ADD CONSTRAINT patient_death_information_patient_id_fkey FOREIGN KEY (patient_id) REFERENCES public.patient_information(id) ON DELETE CASCADE;


--
-- Name: patient_death_information patient_death_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_death_information
    ADD CONSTRAINT patient_death_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: patient_identifiers patient_identifiers_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_identifiers
    ADD CONSTRAINT patient_identifiers_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: patient_identifiers patient_identifiers_patient_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_identifiers
    ADD CONSTRAINT patient_identifiers_patient_id_fkey FOREIGN KEY (patient_id) REFERENCES public.patient_information(id) ON DELETE CASCADE;


--
-- Name: patient_identifiers patient_identifiers_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_identifiers
    ADD CONSTRAINT patient_identifiers_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: patient_information patient_information_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_information
    ADD CONSTRAINT patient_information_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: patient_information patient_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_information
    ADD CONSTRAINT patient_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: patient_information patient_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.patient_information
    ADD CONSTRAINT patient_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: permission_profiles permission_profiles_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.permission_profiles
    ADD CONSTRAINT permission_profiles_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: primary_sources primary_sources_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.primary_sources
    ADD CONSTRAINT primary_sources_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: primary_sources primary_sources_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.primary_sources
    ADD CONSTRAINT primary_sources_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: primary_sources primary_sources_source_reporter_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.primary_sources
    ADD CONSTRAINT primary_sources_source_reporter_presave_id_fkey FOREIGN KEY (source_reporter_presave_id) REFERENCES public.reporter_presaves(id) ON DELETE SET NULL;


--
-- Name: primary_sources primary_sources_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.primary_sources
    ADD CONSTRAINT primary_sources_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: principal_authorization_state principal_authorization_state_user_id_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.principal_authorization_state
    ADD CONSTRAINT principal_authorization_state_user_id_organization_id_fkey FOREIGN KEY (user_id, organization_id) REFERENCES public.user_organization_memberships(user_id, organization_id) ON DELETE CASCADE;


--
-- Name: product_presave_active_substances product_presave_active_substances_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presave_active_substances
    ADD CONSTRAINT product_presave_active_substances_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: product_presave_active_substances product_presave_active_substances_product_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presave_active_substances
    ADD CONSTRAINT product_presave_active_substances_product_presave_id_fkey FOREIGN KEY (product_presave_id) REFERENCES public.product_presaves(id) ON DELETE CASCADE;


--
-- Name: product_presave_active_substances product_presave_active_substances_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presave_active_substances
    ADD CONSTRAINT product_presave_active_substances_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: product_presaves product_presaves_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presaves
    ADD CONSTRAINT product_presaves_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: product_presaves product_presaves_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presaves
    ADD CONSTRAINT product_presaves_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: product_presaves product_presaves_receiver_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presaves
    ADD CONSTRAINT product_presaves_receiver_presave_id_fkey FOREIGN KEY (receiver_presave_id) REFERENCES public.receiver_presaves(id) ON DELETE RESTRICT;


--
-- Name: product_presaves product_presaves_sender_org_fk; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presaves
    ADD CONSTRAINT product_presaves_sender_org_fk FOREIGN KEY (sender_presave_id, organization_id) REFERENCES public.sender_presaves(id, organization_id) ON DELETE SET NULL (sender_presave_id);


--
-- Name: product_presaves product_presaves_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.product_presaves
    ADD CONSTRAINT product_presaves_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: reactions reactions_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reactions
    ADD CONSTRAINT reactions_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: reactions reactions_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reactions
    ADD CONSTRAINT reactions_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: reactions reactions_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reactions
    ADD CONSTRAINT reactions_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: receiver_presave_consignees receiver_presave_consignees_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_consignees
    ADD CONSTRAINT receiver_presave_consignees_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: receiver_presave_consignees receiver_presave_consignees_receiver_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_consignees
    ADD CONSTRAINT receiver_presave_consignees_receiver_presave_id_fkey FOREIGN KEY (receiver_presave_id) REFERENCES public.receiver_presaves(id) ON DELETE CASCADE;


--
-- Name: receiver_presave_consignees receiver_presave_consignees_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_consignees
    ADD CONSTRAINT receiver_presave_consignees_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: receiver_presave_routes receiver_presave_routes_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_routes
    ADD CONSTRAINT receiver_presave_routes_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: receiver_presave_routes receiver_presave_routes_receiver_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_routes
    ADD CONSTRAINT receiver_presave_routes_receiver_presave_id_fkey FOREIGN KEY (receiver_presave_id) REFERENCES public.receiver_presaves(id) ON DELETE CASCADE;


--
-- Name: receiver_presave_routes receiver_presave_routes_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presave_routes
    ADD CONSTRAINT receiver_presave_routes_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: receiver_presaves receiver_presaves_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presaves
    ADD CONSTRAINT receiver_presaves_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: receiver_presaves receiver_presaves_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presaves
    ADD CONSTRAINT receiver_presaves_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: receiver_presaves receiver_presaves_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_presaves
    ADD CONSTRAINT receiver_presaves_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: relatedness_assessments relatedness_assessments_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.relatedness_assessments
    ADD CONSTRAINT relatedness_assessments_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: relatedness_assessments relatedness_assessments_drug_reaction_assessment_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.relatedness_assessments
    ADD CONSTRAINT relatedness_assessments_drug_reaction_assessment_id_fkey FOREIGN KEY (drug_reaction_assessment_id) REFERENCES public.drug_reaction_assessments(id) ON DELETE CASCADE;


--
-- Name: relatedness_assessments relatedness_assessments_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.relatedness_assessments
    ADD CONSTRAINT relatedness_assessments_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: reported_causes_of_death reported_causes_of_death_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reported_causes_of_death
    ADD CONSTRAINT reported_causes_of_death_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: reported_causes_of_death reported_causes_of_death_death_info_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reported_causes_of_death
    ADD CONSTRAINT reported_causes_of_death_death_info_id_fkey FOREIGN KEY (death_info_id) REFERENCES public.patient_death_information(id) ON DELETE CASCADE;


--
-- Name: reported_causes_of_death reported_causes_of_death_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reported_causes_of_death
    ADD CONSTRAINT reported_causes_of_death_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: reporter_presaves reporter_presaves_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reporter_presaves
    ADD CONSTRAINT reporter_presaves_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: reporter_presaves reporter_presaves_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reporter_presaves
    ADD CONSTRAINT reporter_presaves_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: reporter_presaves reporter_presaves_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reporter_presaves
    ADD CONSTRAINT reporter_presaves_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: role_grants role_grants_grant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_grants
    ADD CONSTRAINT role_grants_grant_id_fkey FOREIGN KEY (grant_id) REFERENCES public.authorization_grant_catalog(grant_id) ON DELETE RESTRICT;


--
-- Name: role_grants role_grants_role_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_grants
    ADD CONSTRAINT role_grants_role_id_fkey FOREIGN KEY (role_id) REFERENCES public.authorization_roles(id) ON DELETE CASCADE;


--
-- Name: safety_report_identification safety_report_identification_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.safety_report_identification
    ADD CONSTRAINT safety_report_identification_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: safety_report_identification safety_report_identification_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.safety_report_identification
    ADD CONSTRAINT safety_report_identification_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: safety_report_identification safety_report_identification_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.safety_report_identification
    ADD CONSTRAINT safety_report_identification_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_diagnoses sender_diagnoses_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_diagnoses
    ADD CONSTRAINT sender_diagnoses_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_diagnoses sender_diagnoses_narrative_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_diagnoses
    ADD CONSTRAINT sender_diagnoses_narrative_id_fkey FOREIGN KEY (narrative_id) REFERENCES public.narrative_information(id) ON DELETE CASCADE;


--
-- Name: sender_diagnoses sender_diagnoses_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_diagnoses
    ADD CONSTRAINT sender_diagnoses_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_information sender_information_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_information
    ADD CONSTRAINT sender_information_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: sender_information sender_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_information
    ADD CONSTRAINT sender_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_information sender_information_source_sender_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_information
    ADD CONSTRAINT sender_information_source_sender_presave_id_fkey FOREIGN KEY (source_sender_presave_id) REFERENCES public.sender_presaves(id) ON DELETE SET NULL;


--
-- Name: sender_information sender_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_information
    ADD CONSTRAINT sender_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_presave_gateways sender_presave_gateways_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_gateways
    ADD CONSTRAINT sender_presave_gateways_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_presave_gateways sender_presave_gateways_sender_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_gateways
    ADD CONSTRAINT sender_presave_gateways_sender_presave_id_fkey FOREIGN KEY (sender_presave_id) REFERENCES public.sender_presaves(id) ON DELETE CASCADE;


--
-- Name: sender_presave_gateways sender_presave_gateways_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_gateways
    ADD CONSTRAINT sender_presave_gateways_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_presave_responsible_persons sender_presave_responsible_persons_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_responsible_persons
    ADD CONSTRAINT sender_presave_responsible_persons_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_presave_responsible_persons sender_presave_responsible_persons_sender_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_responsible_persons
    ADD CONSTRAINT sender_presave_responsible_persons_sender_presave_id_fkey FOREIGN KEY (sender_presave_id) REFERENCES public.sender_presaves(id) ON DELETE CASCADE;


--
-- Name: sender_presave_responsible_persons sender_presave_responsible_persons_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presave_responsible_persons
    ADD CONSTRAINT sender_presave_responsible_persons_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_presaves sender_presaves_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presaves
    ADD CONSTRAINT sender_presaves_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: sender_presaves sender_presaves_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presaves
    ADD CONSTRAINT sender_presaves_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: sender_presaves sender_presaves_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sender_presaves
    ADD CONSTRAINT sender_presaves_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: source_documents source_documents_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.source_documents
    ADD CONSTRAINT source_documents_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: source_documents source_documents_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.source_documents
    ADD CONSTRAINT source_documents_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: source_documents source_documents_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.source_documents
    ADD CONSTRAINT source_documents_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_fda_cross_reported_inds study_fda_cross_reported_inds_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_fda_cross_reported_inds
    ADD CONSTRAINT study_fda_cross_reported_inds_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_fda_cross_reported_inds study_fda_cross_reported_inds_study_information_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_fda_cross_reported_inds
    ADD CONSTRAINT study_fda_cross_reported_inds_study_information_id_fkey FOREIGN KEY (study_information_id) REFERENCES public.study_information(id) ON DELETE CASCADE;


--
-- Name: study_fda_cross_reported_inds study_fda_cross_reported_inds_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_fda_cross_reported_inds
    ADD CONSTRAINT study_fda_cross_reported_inds_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_information study_information_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_information
    ADD CONSTRAINT study_information_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: study_information study_information_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_information
    ADD CONSTRAINT study_information_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_information study_information_source_study_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_information
    ADD CONSTRAINT study_information_source_study_presave_id_fkey FOREIGN KEY (source_study_presave_id) REFERENCES public.study_presaves(id) ON DELETE SET NULL;


--
-- Name: study_information study_information_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_information
    ADD CONSTRAINT study_information_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presave_fda_cross_reported_ind_numbers study_presave_fda_cross_reported_ind_numb_study_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_fda_cross_reported_ind_numbers
    ADD CONSTRAINT study_presave_fda_cross_reported_ind_numb_study_presave_id_fkey FOREIGN KEY (study_presave_id) REFERENCES public.study_presaves(id) ON DELETE CASCADE;


--
-- Name: study_presave_fda_cross_reported_ind_numbers study_presave_fda_cross_reported_ind_numbers_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_fda_cross_reported_ind_numbers
    ADD CONSTRAINT study_presave_fda_cross_reported_ind_numbers_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presave_fda_cross_reported_ind_numbers study_presave_fda_cross_reported_ind_numbers_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_fda_cross_reported_ind_numbers
    ADD CONSTRAINT study_presave_fda_cross_reported_ind_numbers_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presave_products study_presave_products_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_products
    ADD CONSTRAINT study_presave_products_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presave_products study_presave_products_product_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_products
    ADD CONSTRAINT study_presave_products_product_presave_id_fkey FOREIGN KEY (product_presave_id) REFERENCES public.product_presaves(id) ON DELETE SET NULL;


--
-- Name: study_presave_products study_presave_products_study_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_products
    ADD CONSTRAINT study_presave_products_study_presave_id_fkey FOREIGN KEY (study_presave_id) REFERENCES public.study_presaves(id) ON DELETE CASCADE;


--
-- Name: study_presave_products study_presave_products_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_products
    ADD CONSTRAINT study_presave_products_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presave_registration_numbers study_presave_registration_numbers_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_registration_numbers
    ADD CONSTRAINT study_presave_registration_numbers_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presave_registration_numbers study_presave_registration_numbers_study_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_registration_numbers
    ADD CONSTRAINT study_presave_registration_numbers_study_presave_id_fkey FOREIGN KEY (study_presave_id) REFERENCES public.study_presaves(id) ON DELETE CASCADE;


--
-- Name: study_presave_registration_numbers study_presave_registration_numbers_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_registration_numbers
    ADD CONSTRAINT study_presave_registration_numbers_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presave_reporters study_presave_reporters_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_reporters
    ADD CONSTRAINT study_presave_reporters_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presave_reporters study_presave_reporters_reporter_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_reporters
    ADD CONSTRAINT study_presave_reporters_reporter_presave_id_fkey FOREIGN KEY (reporter_presave_id) REFERENCES public.reporter_presaves(id) ON DELETE SET NULL;


--
-- Name: study_presave_reporters study_presave_reporters_study_presave_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_reporters
    ADD CONSTRAINT study_presave_reporters_study_presave_id_fkey FOREIGN KEY (study_presave_id) REFERENCES public.study_presaves(id) ON DELETE CASCADE;


--
-- Name: study_presave_reporters study_presave_reporters_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presave_reporters
    ADD CONSTRAINT study_presave_reporters_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presaves study_presaves_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presaves
    ADD CONSTRAINT study_presaves_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_presaves study_presaves_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presaves
    ADD CONSTRAINT study_presaves_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE RESTRICT;


--
-- Name: study_presaves study_presaves_product_org_fk; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presaves
    ADD CONSTRAINT study_presaves_product_org_fk FOREIGN KEY (product_presave_id, organization_id) REFERENCES public.product_presaves(id, organization_id) ON DELETE SET NULL (product_presave_id);


--
-- Name: study_presaves study_presaves_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_presaves
    ADD CONSTRAINT study_presaves_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_registration_numbers study_registration_numbers_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_registration_numbers
    ADD CONSTRAINT study_registration_numbers_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: study_registration_numbers study_registration_numbers_study_information_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_registration_numbers
    ADD CONSTRAINT study_registration_numbers_study_information_id_fkey FOREIGN KEY (study_information_id) REFERENCES public.study_information(id) ON DELETE CASCADE;


--
-- Name: study_registration_numbers study_registration_numbers_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.study_registration_numbers
    ADD CONSTRAINT study_registration_numbers_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: submission_acks submission_acks_submission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_acks
    ADD CONSTRAINT submission_acks_submission_id_fkey FOREIGN KEY (submission_id) REFERENCES public.case_submissions(id) ON DELETE CASCADE;


--
-- Name: submission_dispatch_state submission_dispatch_state_submission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_dispatch_state
    ADD CONSTRAINT submission_dispatch_state_submission_id_fkey FOREIGN KEY (submission_id) REFERENCES public.case_submissions(id) ON DELETE CASCADE;


--
-- Name: submission_events submission_events_submission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_events
    ADD CONSTRAINT submission_events_submission_id_fkey FOREIGN KEY (submission_id) REFERENCES public.case_submissions(id) ON DELETE CASCADE;


--
-- Name: submission_idempotency submission_idempotency_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_idempotency
    ADD CONSTRAINT submission_idempotency_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: submission_idempotency submission_idempotency_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_idempotency
    ADD CONSTRAINT submission_idempotency_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id);


--
-- Name: submission_idempotency submission_idempotency_submission_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.submission_idempotency
    ADD CONSTRAINT submission_idempotency_submission_id_fkey FOREIGN KEY (submission_id) REFERENCES public.case_submissions(id) ON DELETE CASCADE;


--
-- Name: test_results test_results_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.test_results
    ADD CONSTRAINT test_results_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: test_results test_results_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.test_results
    ADD CONSTRAINT test_results_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: test_results test_results_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.test_results
    ADD CONSTRAINT test_results_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE RESTRICT;


--
-- Name: user_organization_memberships user_organization_memberships_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_organization_memberships
    ADD CONSTRAINT user_organization_memberships_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: user_organization_memberships user_organization_memberships_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_organization_memberships
    ADD CONSTRAINT user_organization_memberships_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: user_organization_memberships user_organization_memberships_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_organization_memberships
    ADD CONSTRAINT user_organization_memberships_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: user_organization_memberships user_organization_memberships_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_organization_memberships
    ADD CONSTRAINT user_organization_memberships_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_role_assignments user_role_assignments_role_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_role_assignments
    ADD CONSTRAINT user_role_assignments_role_id_fkey FOREIGN KEY (role_id) REFERENCES public.authorization_roles(id) ON DELETE RESTRICT;


--
-- Name: user_role_assignments user_role_assignments_user_id_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_role_assignments
    ADD CONSTRAINT user_role_assignments_user_id_organization_id_fkey FOREIGN KEY (user_id, organization_id) REFERENCES public.user_organization_memberships(user_id, organization_id) ON DELETE CASCADE;


--
-- Name: xml_export_history xml_export_history_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.xml_export_history
    ADD CONSTRAINT xml_export_history_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE CASCADE;


--
-- Name: xml_export_history xml_export_history_exported_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.xml_export_history
    ADD CONSTRAINT xml_export_history_exported_by_fkey FOREIGN KEY (exported_by) REFERENCES public.users(id);


--
-- Name: xml_import_history xml_import_history_case_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.xml_import_history
    ADD CONSTRAINT xml_import_history_case_id_fkey FOREIGN KEY (case_id) REFERENCES public.cases(id) ON DELETE SET NULL;


--
-- Name: xml_import_history xml_import_history_uploaded_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.xml_import_history
    ADD CONSTRAINT xml_import_history_uploaded_by_fkey FOREIGN KEY (uploaded_by) REFERENCES public.users(id);


--
-- Name: app_settings; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.app_settings ENABLE ROW LEVEL SECURITY;

--
-- Name: app_settings app_settings_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY app_settings_org_isolation ON public.app_settings TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: audit_logs; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.audit_logs ENABLE ROW LEVEL SECURITY;

--
-- Name: audit_logs audit_logs_append_only; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY audit_logs_append_only ON public.audit_logs FOR INSERT TO e2br3_app_role WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: audit_logs audit_logs_no_modify; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY audit_logs_no_modify ON public.audit_logs TO e2br3_app_role USING (false);


--
-- Name: audit_logs audit_logs_read_for_admin_manager; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY audit_logs_read_for_admin_manager ON public.audit_logs FOR SELECT TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: audit_logs audit_logs_read_for_auditors; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY audit_logs_read_for_auditors ON public.audit_logs FOR SELECT TO e2br3_auditor_role USING (true);


--
-- Name: authorization_audit_events; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.authorization_audit_events ENABLE ROW LEVEL SECURITY;

--
-- Name: authorization_audit_events authorization_audit_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY authorization_audit_insert ON public.authorization_audit_events FOR INSERT TO e2br3_app_role WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: autopsy_causes_of_death; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.autopsy_causes_of_death ENABLE ROW LEVEL SECURITY;

--
-- Name: autopsy_causes_of_death autopsy_causes_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY autopsy_causes_via_case ON public.autopsy_causes_of_death TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM ((public.patient_death_information pdi
     JOIN public.patient_information pi ON ((pi.id = pdi.patient_id)))
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pdi.id = autopsy_causes_of_death.death_info_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM ((public.patient_death_information pdi
     JOIN public.patient_information pi ON ((pi.id = pdi.patient_id)))
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pdi.id = autopsy_causes_of_death.death_info_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: case_e2b_field_notations; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.case_e2b_field_notations ENABLE ROW LEVEL SECURITY;

--
-- Name: case_e2b_field_notations case_e2b_field_notations_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY case_e2b_field_notations_via_case ON public.case_e2b_field_notations TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_e2b_field_notations.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_e2b_field_notations.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: case_field_notations; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.case_field_notations ENABLE ROW LEVEL SECURITY;

--
-- Name: case_field_notations case_field_notations_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY case_field_notations_via_case ON public.case_field_notations TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_field_notations.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_field_notations.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: case_submissions; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.case_submissions ENABLE ROW LEVEL SECURITY;

--
-- Name: case_submissions case_submissions_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY case_submissions_via_case ON public.case_submissions TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_submissions.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_submissions.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: case_summary_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.case_summary_information ENABLE ROW LEVEL SECURITY;

--
-- Name: case_summary_information case_summary_information_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY case_summary_information_via_case ON public.case_summary_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.narrative_information ni
     JOIN public.cases c ON ((c.id = ni.case_id)))
  WHERE ((ni.id = case_summary_information.narrative_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.narrative_information ni
     JOIN public.cases c ON ((c.id = ni.case_id)))
  WHERE ((ni.id = case_summary_information.narrative_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: case_validation_summaries; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.case_validation_summaries ENABLE ROW LEVEL SECURITY;

--
-- Name: case_validation_summaries case_validation_summaries_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY case_validation_summaries_via_case ON public.case_validation_summaries TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_validation_summaries.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_validation_summaries.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: case_versions; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.case_versions ENABLE ROW LEVEL SECURITY;

--
-- Name: case_versions case_versions_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY case_versions_via_case ON public.case_versions TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_versions.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_versions.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: case_workflow_events; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.case_workflow_events ENABLE ROW LEVEL SECURITY;

--
-- Name: case_workflow_events case_workflow_events_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY case_workflow_events_via_case ON public.case_workflow_events TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_workflow_events.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = case_workflow_events.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: cases; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.cases ENABLE ROW LEVEL SECURITY;

--
-- Name: cases cases_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY cases_org_isolation ON public.cases TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: controlled_terminology_terms; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.controlled_terminology_terms ENABLE ROW LEVEL SECURITY;

--
-- Name: controlled_terminology_terms controlled_terminology_terms_delete; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY controlled_terminology_terms_delete ON public.controlled_terminology_terms FOR DELETE TO e2br3_app_role USING (public.is_current_user_admin());


--
-- Name: controlled_terminology_terms controlled_terminology_terms_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY controlled_terminology_terms_insert ON public.controlled_terminology_terms FOR INSERT TO e2br3_app_role WITH CHECK (public.is_current_user_admin());


--
-- Name: controlled_terminology_terms controlled_terminology_terms_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY controlled_terminology_terms_read ON public.controlled_terminology_terms FOR SELECT TO e2br3_app_role USING ((public.is_current_user_admin() OR (((dictionary)::text <> 'whodrug'::text) AND (active = true)) OR (((dictionary)::text = 'whodrug'::text) AND (EXISTS ( SELECT 1
   FROM public.terminology_releases r
  WHERE (((r.dictionary)::text = 'whodrug'::text) AND ((r.version)::text = (controlled_terminology_terms.version)::text) AND ((r.language)::text = (controlled_terminology_terms.language)::text) AND ((r.status)::text = 'active'::text)))))));


--
-- Name: controlled_terminology_terms controlled_terminology_terms_update; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY controlled_terminology_terms_update ON public.controlled_terminology_terms FOR UPDATE TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: dashboard_notices; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.dashboard_notices ENABLE ROW LEVEL SECURITY;

--
-- Name: dashboard_notices dashboard_notices_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY dashboard_notices_org_isolation ON public.dashboard_notices TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: documents_held_by_sender; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.documents_held_by_sender ENABLE ROW LEVEL SECURITY;

--
-- Name: documents_held_by_sender documents_held_by_sender_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY documents_held_by_sender_via_case ON public.documents_held_by_sender TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = documents_held_by_sender.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = documents_held_by_sender.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: dosage_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.dosage_information ENABLE ROW LEVEL SECURITY;

--
-- Name: dosage_information dosage_information_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY dosage_information_via_case ON public.dosage_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = dosage_information.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = dosage_information.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: drug_active_substances; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.drug_active_substances ENABLE ROW LEVEL SECURITY;

--
-- Name: drug_active_substances drug_active_substances_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY drug_active_substances_via_case ON public.drug_active_substances TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = drug_active_substances.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = drug_active_substances.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: drug_device_characteristics; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.drug_device_characteristics ENABLE ROW LEVEL SECURITY;

--
-- Name: drug_device_characteristics drug_device_characteristics_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY drug_device_characteristics_via_case ON public.drug_device_characteristics TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = drug_device_characteristics.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = drug_device_characteristics.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: drug_indications; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.drug_indications ENABLE ROW LEVEL SECURITY;

--
-- Name: drug_indications drug_indications_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY drug_indications_via_case ON public.drug_indications TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = drug_indications.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = drug_indications.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: drug_information drug_info_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY drug_info_via_case ON public.drug_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = drug_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = drug_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: drug_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.drug_information ENABLE ROW LEVEL SECURITY;

--
-- Name: drug_reaction_assessments; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.drug_reaction_assessments ENABLE ROW LEVEL SECURITY;

--
-- Name: drug_reaction_assessments drug_reaction_assessments_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY drug_reaction_assessments_via_case ON public.drug_reaction_assessments TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = drug_reaction_assessments.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = drug_reaction_assessments.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: e2b_code_lists; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.e2b_code_lists ENABLE ROW LEVEL SECURITY;

--
-- Name: e2b_code_lists e2b_code_lists_delete; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY e2b_code_lists_delete ON public.e2b_code_lists FOR DELETE TO e2br3_app_role USING (public.is_current_user_admin());


--
-- Name: e2b_code_lists e2b_code_lists_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY e2b_code_lists_insert ON public.e2b_code_lists FOR INSERT TO e2br3_app_role WITH CHECK (public.is_current_user_admin());


--
-- Name: e2b_code_lists e2b_code_lists_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY e2b_code_lists_read ON public.e2b_code_lists FOR SELECT TO e2br3_app_role USING (((active = true) OR public.is_current_user_admin()));


--
-- Name: e2b_code_lists e2b_code_lists_update; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY e2b_code_lists_update ON public.e2b_code_lists FOR UPDATE TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: e_signatures; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.e_signatures ENABLE ROW LEVEL SECURITY;

--
-- Name: e_signatures e_signatures_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY e_signatures_via_case ON public.e_signatures TO e2br3_app_role USING (((case_id IS NULL) OR (EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = e_signatures.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))))) WITH CHECK (((case_id IS NULL) OR (EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = e_signatures.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))));


--
-- Name: fda_device_codes; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.fda_device_codes ENABLE ROW LEVEL SECURITY;

--
-- Name: fda_device_codes fda_device_codes_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY fda_device_codes_via_case ON public.fda_device_codes TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM ((public.fda_device_information fdi
     JOIN public.drug_information di ON ((di.id = fdi.drug_id)))
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((fdi.id = fda_device_codes.device_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM ((public.fda_device_information fdi
     JOIN public.drug_information di ON ((di.id = fdi.drug_id)))
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((fdi.id = fda_device_codes.device_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: fda_device_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.fda_device_information ENABLE ROW LEVEL SECURITY;

--
-- Name: fda_device_information fda_device_information_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY fda_device_information_via_case ON public.fda_device_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = fda_device_information.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.drug_information di
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((di.id = fda_device_information.drug_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: iso_countries; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.iso_countries ENABLE ROW LEVEL SECURITY;

--
-- Name: iso_countries iso_countries_delete; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY iso_countries_delete ON public.iso_countries FOR DELETE TO e2br3_app_role USING (public.is_current_user_admin());


--
-- Name: iso_countries iso_countries_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY iso_countries_insert ON public.iso_countries FOR INSERT TO e2br3_app_role WITH CHECK (public.is_current_user_admin());


--
-- Name: iso_countries iso_countries_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY iso_countries_read ON public.iso_countries FOR SELECT TO e2br3_app_role USING (((active = true) OR public.is_current_user_admin()));


--
-- Name: iso_countries iso_countries_update; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY iso_countries_update ON public.iso_countries FOR UPDATE TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: linked_report_numbers; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.linked_report_numbers ENABLE ROW LEVEL SECURITY;

--
-- Name: linked_report_numbers linked_reports_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY linked_reports_via_case ON public.linked_report_numbers TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = linked_report_numbers.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = linked_report_numbers.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: literature_references; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.literature_references ENABLE ROW LEVEL SECURITY;

--
-- Name: literature_references literature_refs_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY literature_refs_via_case ON public.literature_references TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = literature_references.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = literature_references.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: meddra_terms; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.meddra_terms ENABLE ROW LEVEL SECURITY;

--
-- Name: meddra_terms meddra_terms_delete; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY meddra_terms_delete ON public.meddra_terms FOR DELETE TO e2br3_app_role USING (public.is_current_user_admin());


--
-- Name: meddra_terms meddra_terms_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY meddra_terms_insert ON public.meddra_terms FOR INSERT TO e2br3_app_role WITH CHECK (public.is_current_user_admin());


--
-- Name: meddra_terms meddra_terms_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY meddra_terms_read ON public.meddra_terms FOR SELECT TO e2br3_app_role USING ((public.is_current_user_admin() OR (EXISTS ( SELECT 1
   FROM public.terminology_releases r
  WHERE (((r.dictionary)::text = 'meddra'::text) AND ((r.version)::text = (meddra_terms.version)::text) AND ((r.language)::text = (meddra_terms.language)::text) AND ((r.status)::text = 'active'::text))))));


--
-- Name: meddra_terms meddra_terms_update; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY meddra_terms_update ON public.meddra_terms FOR UPDATE TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: medical_history_episodes; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.medical_history_episodes ENABLE ROW LEVEL SECURITY;

--
-- Name: medical_history_episodes medical_history_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY medical_history_via_case ON public.medical_history_episodes TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = medical_history_episodes.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = medical_history_episodes.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: mfds_product_substances; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.mfds_product_substances ENABLE ROW LEVEL SECURITY;

--
-- Name: mfds_product_substances mfds_product_substances_delete; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY mfds_product_substances_delete ON public.mfds_product_substances FOR DELETE TO e2br3_app_role USING (public.is_current_user_admin());


--
-- Name: mfds_product_substances mfds_product_substances_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY mfds_product_substances_insert ON public.mfds_product_substances FOR INSERT TO e2br3_app_role WITH CHECK (public.is_current_user_admin());


--
-- Name: mfds_product_substances mfds_product_substances_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY mfds_product_substances_read ON public.mfds_product_substances FOR SELECT TO e2br3_app_role USING (((active = true) OR public.is_current_user_admin()));


--
-- Name: mfds_product_substances mfds_product_substances_update; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY mfds_product_substances_update ON public.mfds_product_substances FOR UPDATE TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: mfds_products; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.mfds_products ENABLE ROW LEVEL SECURITY;

--
-- Name: mfds_products mfds_products_delete; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY mfds_products_delete ON public.mfds_products FOR DELETE TO e2br3_app_role USING (public.is_current_user_admin());


--
-- Name: mfds_products mfds_products_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY mfds_products_insert ON public.mfds_products FOR INSERT TO e2br3_app_role WITH CHECK (public.is_current_user_admin());


--
-- Name: mfds_products mfds_products_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY mfds_products_read ON public.mfds_products FOR SELECT TO e2br3_app_role USING (((active = true) OR public.is_current_user_admin()));


--
-- Name: mfds_products mfds_products_update; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY mfds_products_update ON public.mfds_products FOR UPDATE TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: narrative_information narrative_info_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY narrative_info_via_case ON public.narrative_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = narrative_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = narrative_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: narrative_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.narrative_information ENABLE ROW LEVEL SECURITY;

--
-- Name: narrative_presaves; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.narrative_presaves ENABLE ROW LEVEL SECURITY;

--
-- Name: narrative_presaves narrative_presaves_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY narrative_presaves_org_isolation ON public.narrative_presaves TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: organization_as2_profiles; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.organization_as2_profiles ENABLE ROW LEVEL SECURITY;

--
-- Name: organization_as2_profiles organization_as2_profiles_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY organization_as2_profiles_org_isolation ON public.organization_as2_profiles TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: organizations; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.organizations ENABLE ROW LEVEL SECURITY;

--
-- Name: organizations orgs_modify; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY orgs_modify ON public.organizations TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: organizations orgs_select; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY orgs_select ON public.organizations FOR SELECT TO e2br3_app_role USING (((id = public.current_organization_id()) OR public.is_current_user_admin() OR (EXISTS ( SELECT 1
   FROM public.user_organization_memberships membership
  WHERE ((membership.organization_id = organizations.id) AND (membership.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND (membership.active = true))))));


--
-- Name: other_case_identifiers; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.other_case_identifiers ENABLE ROW LEVEL SECURITY;

--
-- Name: other_case_identifiers other_case_ids_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY other_case_ids_via_case ON public.other_case_identifiers TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = other_case_identifiers.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = other_case_identifiers.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: parent_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.parent_information ENABLE ROW LEVEL SECURITY;

--
-- Name: parent_information parent_information_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY parent_information_via_case ON public.parent_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = parent_information.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = parent_information.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: parent_medical_history; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.parent_medical_history ENABLE ROW LEVEL SECURITY;

--
-- Name: parent_medical_history parent_medical_history_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY parent_medical_history_via_case ON public.parent_medical_history TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM ((public.parent_information pi2
     JOIN public.patient_information pi ON ((pi.id = pi2.patient_id)))
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi2.id = parent_medical_history.parent_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM ((public.parent_information pi2
     JOIN public.patient_information pi ON ((pi.id = pi2.patient_id)))
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi2.id = parent_medical_history.parent_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: parent_past_drug_history; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.parent_past_drug_history ENABLE ROW LEVEL SECURITY;

--
-- Name: parent_past_drug_history parent_past_drug_history_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY parent_past_drug_history_via_case ON public.parent_past_drug_history TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM ((public.parent_information pi2
     JOIN public.patient_information pi ON ((pi.id = pi2.patient_id)))
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi2.id = parent_past_drug_history.parent_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM ((public.parent_information pi2
     JOIN public.patient_information pi ON ((pi.id = pi2.patient_id)))
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi2.id = parent_past_drug_history.parent_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: past_drug_history; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.past_drug_history ENABLE ROW LEVEL SECURITY;

--
-- Name: past_drug_history past_drug_history_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY past_drug_history_via_case ON public.past_drug_history TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = past_drug_history.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = past_drug_history.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: patient_death_information patient_death_info_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY patient_death_info_via_case ON public.patient_death_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = patient_death_information.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = patient_death_information.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: patient_death_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.patient_death_information ENABLE ROW LEVEL SECURITY;

--
-- Name: patient_identifiers; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.patient_identifiers ENABLE ROW LEVEL SECURITY;

--
-- Name: patient_identifiers patient_identifiers_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY patient_identifiers_via_case ON public.patient_identifiers TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = patient_identifiers.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.patient_information pi
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pi.id = patient_identifiers.patient_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: patient_information patient_info_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY patient_info_via_case ON public.patient_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = patient_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = patient_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: patient_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.patient_information ENABLE ROW LEVEL SECURITY;

--
-- Name: permission_profiles; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.permission_profiles ENABLE ROW LEVEL SECURITY;

--
-- Name: permission_profiles permission_profiles_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY permission_profiles_org_isolation ON public.permission_profiles TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: primary_sources; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.primary_sources ENABLE ROW LEVEL SECURITY;

--
-- Name: primary_sources primary_sources_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY primary_sources_via_case ON public.primary_sources TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = primary_sources.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = primary_sources.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: product_presave_active_substances; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.product_presave_active_substances ENABLE ROW LEVEL SECURITY;

--
-- Name: product_presave_active_substances product_presave_active_substances_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY product_presave_active_substances_via_parent ON public.product_presave_active_substances TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.product_presaves p
  WHERE ((p.id = product_presave_active_substances.product_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.product_presaves p
  WHERE ((p.id = product_presave_active_substances.product_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: product_presaves; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.product_presaves ENABLE ROW LEVEL SECURITY;

--
-- Name: product_presaves product_presaves_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY product_presaves_org_isolation ON public.product_presaves TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: reactions; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.reactions ENABLE ROW LEVEL SECURITY;

--
-- Name: reactions reactions_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY reactions_via_case ON public.reactions TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = reactions.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = reactions.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: receiver_presave_consignees; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.receiver_presave_consignees ENABLE ROW LEVEL SECURITY;

--
-- Name: receiver_presave_consignees receiver_presave_consignees_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY receiver_presave_consignees_via_parent ON public.receiver_presave_consignees TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.receiver_presaves p
  WHERE ((p.id = receiver_presave_consignees.receiver_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.receiver_presaves p
  WHERE ((p.id = receiver_presave_consignees.receiver_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: receiver_presave_routes; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.receiver_presave_routes ENABLE ROW LEVEL SECURITY;

--
-- Name: receiver_presave_routes receiver_presave_routes_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY receiver_presave_routes_via_parent ON public.receiver_presave_routes TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.receiver_presaves p
  WHERE ((p.id = receiver_presave_routes.receiver_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.receiver_presaves p
  WHERE ((p.id = receiver_presave_routes.receiver_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: receiver_presaves; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.receiver_presaves ENABLE ROW LEVEL SECURITY;

--
-- Name: receiver_presaves receiver_presaves_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY receiver_presaves_org_isolation ON public.receiver_presaves TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: relatedness_assessments; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.relatedness_assessments ENABLE ROW LEVEL SECURITY;

--
-- Name: relatedness_assessments relatedness_assessments_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY relatedness_assessments_via_case ON public.relatedness_assessments TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM ((public.drug_reaction_assessments dra
     JOIN public.drug_information di ON ((di.id = dra.drug_id)))
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((dra.id = relatedness_assessments.drug_reaction_assessment_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM ((public.drug_reaction_assessments dra
     JOIN public.drug_information di ON ((di.id = dra.drug_id)))
     JOIN public.cases c ON ((c.id = di.case_id)))
  WHERE ((dra.id = relatedness_assessments.drug_reaction_assessment_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: reported_causes_of_death; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.reported_causes_of_death ENABLE ROW LEVEL SECURITY;

--
-- Name: reported_causes_of_death reported_causes_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY reported_causes_via_case ON public.reported_causes_of_death TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM ((public.patient_death_information pdi
     JOIN public.patient_information pi ON ((pi.id = pdi.patient_id)))
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pdi.id = reported_causes_of_death.death_info_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM ((public.patient_death_information pdi
     JOIN public.patient_information pi ON ((pi.id = pdi.patient_id)))
     JOIN public.cases c ON ((c.id = pi.case_id)))
  WHERE ((pdi.id = reported_causes_of_death.death_info_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: reporter_presaves; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.reporter_presaves ENABLE ROW LEVEL SECURITY;

--
-- Name: reporter_presaves reporter_presaves_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY reporter_presaves_org_isolation ON public.reporter_presaves TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: safety_report_identification safety_report_id_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY safety_report_id_via_case ON public.safety_report_identification TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = safety_report_identification.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = safety_report_identification.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: safety_report_identification; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.safety_report_identification ENABLE ROW LEVEL SECURITY;

--
-- Name: sender_diagnoses; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.sender_diagnoses ENABLE ROW LEVEL SECURITY;

--
-- Name: sender_diagnoses sender_diagnoses_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY sender_diagnoses_via_case ON public.sender_diagnoses TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.narrative_information ni
     JOIN public.cases c ON ((c.id = ni.case_id)))
  WHERE ((ni.id = sender_diagnoses.narrative_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.narrative_information ni
     JOIN public.cases c ON ((c.id = ni.case_id)))
  WHERE ((ni.id = sender_diagnoses.narrative_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: sender_information sender_info_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY sender_info_via_case ON public.sender_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = sender_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = sender_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: sender_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.sender_information ENABLE ROW LEVEL SECURITY;

--
-- Name: sender_presave_gateways; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.sender_presave_gateways ENABLE ROW LEVEL SECURITY;

--
-- Name: sender_presave_gateways sender_presave_gateways_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY sender_presave_gateways_via_parent ON public.sender_presave_gateways TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.sender_presaves p
  WHERE ((p.id = sender_presave_gateways.sender_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.sender_presaves p
  WHERE ((p.id = sender_presave_gateways.sender_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: sender_presave_responsible_persons; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.sender_presave_responsible_persons ENABLE ROW LEVEL SECURITY;

--
-- Name: sender_presave_responsible_persons sender_presave_responsible_persons_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY sender_presave_responsible_persons_via_parent ON public.sender_presave_responsible_persons TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.sender_presaves p
  WHERE ((p.id = sender_presave_responsible_persons.sender_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.sender_presaves p
  WHERE ((p.id = sender_presave_responsible_persons.sender_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: sender_presaves; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.sender_presaves ENABLE ROW LEVEL SECURITY;

--
-- Name: sender_presaves sender_presaves_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY sender_presaves_org_isolation ON public.sender_presaves TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: study_fda_cross_reported_inds; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.study_fda_cross_reported_inds ENABLE ROW LEVEL SECURITY;

--
-- Name: study_fda_cross_reported_inds study_fda_cross_reported_inds_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY study_fda_cross_reported_inds_via_case ON public.study_fda_cross_reported_inds TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.study_information si
     JOIN public.cases c ON ((c.id = si.case_id)))
  WHERE ((si.id = study_fda_cross_reported_inds.study_information_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.study_information si
     JOIN public.cases c ON ((c.id = si.case_id)))
  WHERE ((si.id = study_fda_cross_reported_inds.study_information_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: study_information study_info_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY study_info_via_case ON public.study_information TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = study_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = study_information.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: study_information; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.study_information ENABLE ROW LEVEL SECURITY;

--
-- Name: study_presave_fda_cross_reported_ind_numbers; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.study_presave_fda_cross_reported_ind_numbers ENABLE ROW LEVEL SECURITY;

--
-- Name: study_presave_fda_cross_reported_ind_numbers study_presave_fda_cross_reported_ind_numbers_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY study_presave_fda_cross_reported_ind_numbers_via_parent ON public.study_presave_fda_cross_reported_ind_numbers TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.study_presaves p
  WHERE ((p.id = study_presave_fda_cross_reported_ind_numbers.study_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.study_presaves p
  WHERE ((p.id = study_presave_fda_cross_reported_ind_numbers.study_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: study_presave_products; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.study_presave_products ENABLE ROW LEVEL SECURITY;

--
-- Name: study_presave_products study_presave_products_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY study_presave_products_via_parent ON public.study_presave_products TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.study_presaves p
  WHERE ((p.id = study_presave_products.study_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.study_presaves p
  WHERE ((p.id = study_presave_products.study_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: study_presave_registration_numbers; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.study_presave_registration_numbers ENABLE ROW LEVEL SECURITY;

--
-- Name: study_presave_registration_numbers study_presave_registration_numbers_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY study_presave_registration_numbers_via_parent ON public.study_presave_registration_numbers TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.study_presaves p
  WHERE ((p.id = study_presave_registration_numbers.study_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.study_presaves p
  WHERE ((p.id = study_presave_registration_numbers.study_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: study_presave_reporters; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.study_presave_reporters ENABLE ROW LEVEL SECURITY;

--
-- Name: study_presave_reporters study_presave_reporters_via_parent; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY study_presave_reporters_via_parent ON public.study_presave_reporters TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.study_presaves p
  WHERE ((p.id = study_presave_reporters.study_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.study_presaves p
  WHERE ((p.id = study_presave_reporters.study_presave_id) AND ((p.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: study_presaves; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.study_presaves ENABLE ROW LEVEL SECURITY;

--
-- Name: study_presaves study_presaves_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY study_presaves_org_isolation ON public.study_presaves TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: study_registration_numbers study_reg_numbers_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY study_reg_numbers_via_case ON public.study_registration_numbers TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.study_information si
     JOIN public.cases c ON ((c.id = si.case_id)))
  WHERE ((si.id = study_registration_numbers.study_information_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.study_information si
     JOIN public.cases c ON ((c.id = si.case_id)))
  WHERE ((si.id = study_registration_numbers.study_information_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: study_registration_numbers; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.study_registration_numbers ENABLE ROW LEVEL SECURITY;

--
-- Name: submission_acks; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.submission_acks ENABLE ROW LEVEL SECURITY;

--
-- Name: submission_acks submission_acks_via_submission; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY submission_acks_via_submission ON public.submission_acks TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.case_submissions cs
     JOIN public.cases c ON ((c.id = cs.case_id)))
  WHERE ((cs.id = submission_acks.submission_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.case_submissions cs
     JOIN public.cases c ON ((c.id = cs.case_id)))
  WHERE ((cs.id = submission_acks.submission_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: submission_dispatch_state; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.submission_dispatch_state ENABLE ROW LEVEL SECURITY;

--
-- Name: submission_dispatch_state submission_dispatch_state_via_submission; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY submission_dispatch_state_via_submission ON public.submission_dispatch_state TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.case_submissions cs
     JOIN public.cases c ON ((c.id = cs.case_id)))
  WHERE ((cs.id = submission_dispatch_state.submission_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.case_submissions cs
     JOIN public.cases c ON ((c.id = cs.case_id)))
  WHERE ((cs.id = submission_dispatch_state.submission_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: submission_events; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.submission_events ENABLE ROW LEVEL SECURITY;

--
-- Name: submission_events submission_events_via_submission; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY submission_events_via_submission ON public.submission_events TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM (public.case_submissions cs
     JOIN public.cases c ON ((c.id = cs.case_id)))
  WHERE ((cs.id = submission_events.submission_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM (public.case_submissions cs
     JOIN public.cases c ON ((c.id = cs.case_id)))
  WHERE ((cs.id = submission_events.submission_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: submission_idempotency; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.submission_idempotency ENABLE ROW LEVEL SECURITY;

--
-- Name: submission_idempotency submission_idempotency_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY submission_idempotency_via_case ON public.submission_idempotency TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = submission_idempotency.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = submission_idempotency.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: terminology_releases; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.terminology_releases ENABLE ROW LEVEL SECURITY;

--
-- Name: terminology_releases terminology_releases_delete; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY terminology_releases_delete ON public.terminology_releases FOR DELETE TO e2br3_app_role USING (public.is_current_user_admin());


--
-- Name: terminology_releases terminology_releases_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY terminology_releases_insert ON public.terminology_releases FOR INSERT TO e2br3_app_role WITH CHECK (public.is_current_user_admin());


--
-- Name: terminology_releases terminology_releases_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY terminology_releases_read ON public.terminology_releases FOR SELECT TO e2br3_app_role USING ((((status)::text = ANY (ARRAY[('validated'::character varying)::text, ('approved'::character varying)::text, ('active'::character varying)::text])) OR public.is_current_user_admin()));


--
-- Name: terminology_releases terminology_releases_update; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY terminology_releases_update ON public.terminology_releases FOR UPDATE TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: test_results; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.test_results ENABLE ROW LEVEL SECURITY;

--
-- Name: test_results test_results_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY test_results_via_case ON public.test_results TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = test_results.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = test_results.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: user_organization_memberships; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.user_organization_memberships ENABLE ROW LEVEL SECURITY;

--
-- Name: user_organization_memberships user_organization_memberships_modify; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY user_organization_memberships_modify ON public.user_organization_memberships TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: user_organization_memberships user_organization_memberships_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY user_organization_memberships_read ON public.user_organization_memberships FOR SELECT TO e2br3_app_role USING (((user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) OR public.is_current_user_admin()));


--
-- Name: users; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;

--
-- Name: users users_org_isolation_modify; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY users_org_isolation_modify ON public.users TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin())) WITH CHECK (((organization_id = public.current_organization_id()) OR public.is_current_user_admin()));


--
-- Name: users users_org_isolation_select; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY users_org_isolation_select ON public.users FOR SELECT TO e2br3_app_role USING (((organization_id = public.current_organization_id()) OR public.is_current_user_admin() OR (id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)));


--
-- Name: whodrug_products; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.whodrug_products ENABLE ROW LEVEL SECURITY;

--
-- Name: whodrug_products whodrug_products_delete; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY whodrug_products_delete ON public.whodrug_products FOR DELETE TO e2br3_app_role USING (public.is_current_user_admin());


--
-- Name: whodrug_products whodrug_products_insert; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY whodrug_products_insert ON public.whodrug_products FOR INSERT TO e2br3_app_role WITH CHECK (public.is_current_user_admin());


--
-- Name: whodrug_products whodrug_products_read; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY whodrug_products_read ON public.whodrug_products FOR SELECT TO e2br3_app_role USING ((public.is_current_user_admin() OR (EXISTS ( SELECT 1
   FROM public.terminology_releases r
  WHERE (((r.dictionary)::text = 'whodrug'::text) AND ((r.version)::text = (whodrug_products.version)::text) AND ((r.language)::text = (whodrug_products.language)::text) AND ((r.status)::text = 'active'::text))))));


--
-- Name: whodrug_products whodrug_products_update; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY whodrug_products_update ON public.whodrug_products FOR UPDATE TO e2br3_app_role USING (public.is_current_user_admin()) WITH CHECK (public.is_current_user_admin());


--
-- Name: xml_export_history; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.xml_export_history ENABLE ROW LEVEL SECURITY;

--
-- Name: xml_export_history xml_export_history_via_case; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY xml_export_history_via_case ON public.xml_export_history TO e2br3_app_role USING ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = xml_export_history.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin()))))) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.cases c
  WHERE ((c.id = xml_export_history.case_id) AND ((c.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))));


--
-- Name: xml_import_history; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.xml_import_history ENABLE ROW LEVEL SECURITY;

--
-- Name: xml_import_history xml_import_history_org_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY xml_import_history_org_isolation ON public.xml_import_history TO e2br3_app_role USING (((uploaded_by = public.get_current_user_context()) OR (EXISTS ( SELECT 1
   FROM public.users u
  WHERE ((u.id = xml_import_history.uploaded_by) AND ((u.organization_id = public.current_organization_id()) OR public.is_current_user_admin())))))) WITH CHECK (((uploaded_by = public.get_current_user_context()) OR public.is_current_user_admin()));


--
-- PostgreSQL database dump complete
--

ALTER FUNCTION public.search_active_whodrug_products(text, bigint) OWNER TO e2br3_whodrug_search_owner;
