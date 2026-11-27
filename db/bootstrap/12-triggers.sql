--
-- Name: documents_held_by_sender aa_dirty_c_documents_held_by_sender; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_documents_held_by_sender AFTER INSERT OR DELETE OR UPDATE ON public.documents_held_by_sender FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c();


--
-- Name: linked_report_numbers aa_dirty_c_linked_report_numbers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_linked_report_numbers AFTER INSERT OR DELETE OR UPDATE ON public.linked_report_numbers FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c();


--
-- Name: literature_references aa_dirty_c_literature_references; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_literature_references AFTER INSERT OR DELETE OR UPDATE ON public.literature_references FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c();


--
-- Name: other_case_identifiers aa_dirty_c_other_case_identifiers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_other_case_identifiers AFTER INSERT OR DELETE OR UPDATE ON public.other_case_identifiers FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c();


--
-- Name: primary_sources aa_dirty_c_primary_sources; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_primary_sources AFTER INSERT OR DELETE OR UPDATE ON public.primary_sources FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c();


--
-- Name: safety_report_identification aa_dirty_c_safety_report_identification; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_safety_report_identification AFTER INSERT OR DELETE OR UPDATE ON public.safety_report_identification FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c();


--
-- Name: sender_information aa_dirty_c_sender_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_sender_information AFTER INSERT OR DELETE OR UPDATE ON public.sender_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c();


--
-- Name: study_fda_cross_reported_inds aa_dirty_c_study_fda_cross_reported_inds; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_study_fda_cross_reported_inds AFTER INSERT OR DELETE OR UPDATE ON public.study_fda_cross_reported_inds FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c_from_study_info();


--
-- Name: study_information aa_dirty_c_study_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_study_information AFTER INSERT OR DELETE OR UPDATE ON public.study_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c();


--
-- Name: study_registration_numbers aa_dirty_c_study_registration_numbers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_c_study_registration_numbers AFTER INSERT OR DELETE OR UPDATE ON public.study_registration_numbers FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_c_from_study_info();


--
-- Name: autopsy_causes_of_death aa_dirty_d_autopsy_causes_of_death; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_autopsy_causes_of_death AFTER INSERT OR DELETE OR UPDATE ON public.autopsy_causes_of_death FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_death_info();


--
-- Name: medical_history_episodes aa_dirty_d_medical_history_episodes; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_medical_history_episodes AFTER INSERT OR DELETE OR UPDATE ON public.medical_history_episodes FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_patient();


--
-- Name: parent_information aa_dirty_d_parent_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_parent_information AFTER INSERT OR DELETE OR UPDATE ON public.parent_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_patient();


--
-- Name: parent_medical_history aa_dirty_d_parent_medical_history; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_parent_medical_history AFTER INSERT OR DELETE OR UPDATE ON public.parent_medical_history FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_parent();


--
-- Name: parent_past_drug_history aa_dirty_d_parent_past_drug_history; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_parent_past_drug_history AFTER INSERT OR DELETE OR UPDATE ON public.parent_past_drug_history FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_parent();


--
-- Name: past_drug_history aa_dirty_d_past_drug_history; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_past_drug_history AFTER INSERT OR DELETE OR UPDATE ON public.past_drug_history FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_patient();


--
-- Name: patient_death_information aa_dirty_d_patient_death_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_patient_death_information AFTER INSERT OR DELETE OR UPDATE ON public.patient_death_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_patient();


--
-- Name: patient_identifiers aa_dirty_d_patient_identifiers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_patient_identifiers AFTER INSERT OR DELETE OR UPDATE ON public.patient_identifiers FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_patient();


--
-- Name: patient_information aa_dirty_d_patient_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_patient_information AFTER INSERT OR DELETE OR UPDATE ON public.patient_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d();


--
-- Name: reported_causes_of_death aa_dirty_d_reported_causes_of_death; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_d_reported_causes_of_death AFTER INSERT OR DELETE OR UPDATE ON public.reported_causes_of_death FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_d_from_death_info();


--
-- Name: reactions aa_dirty_e_reactions; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_e_reactions AFTER INSERT OR DELETE OR UPDATE ON public.reactions FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_e();


--
-- Name: test_results aa_dirty_f_test_results; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_f_test_results AFTER INSERT OR DELETE OR UPDATE ON public.test_results FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_f();


--
-- Name: dosage_information aa_dirty_g_dosage_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_g_dosage_information AFTER INSERT OR DELETE OR UPDATE ON public.dosage_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_g_from_drug();


--
-- Name: drug_active_substances aa_dirty_g_drug_active_substances; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_g_drug_active_substances AFTER INSERT OR DELETE OR UPDATE ON public.drug_active_substances FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_g_from_drug();


--
-- Name: drug_device_characteristics aa_dirty_g_drug_device_characteristics; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_g_drug_device_characteristics AFTER INSERT OR DELETE OR UPDATE ON public.drug_device_characteristics FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_g_from_drug();


--
-- Name: drug_indications aa_dirty_g_drug_indications; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_g_drug_indications AFTER INSERT OR DELETE OR UPDATE ON public.drug_indications FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_g_from_drug();


--
-- Name: drug_information aa_dirty_g_drug_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_g_drug_information AFTER INSERT OR DELETE OR UPDATE ON public.drug_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_g();


--
-- Name: drug_reaction_assessments aa_dirty_g_drug_reaction_assessments; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_g_drug_reaction_assessments AFTER INSERT OR DELETE OR UPDATE ON public.drug_reaction_assessments FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_g_from_drug();


--
-- Name: relatedness_assessments aa_dirty_g_relatedness_assessments; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_g_relatedness_assessments AFTER INSERT OR DELETE OR UPDATE ON public.relatedness_assessments FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_g_from_drug();


--
-- Name: case_summary_information aa_dirty_h_case_summary_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_h_case_summary_information AFTER INSERT OR DELETE OR UPDATE ON public.case_summary_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_h_from_narrative();


--
-- Name: narrative_information aa_dirty_h_narrative_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_h_narrative_information AFTER INSERT OR DELETE OR UPDATE ON public.narrative_information FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_h();


--
-- Name: sender_diagnoses aa_dirty_h_sender_diagnoses; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER aa_dirty_h_sender_diagnoses AFTER INSERT OR DELETE OR UPDATE ON public.sender_diagnoses FOR EACH ROW EXECUTE FUNCTION public.mark_case_dirty_h_from_narrative();


--
-- Name: autopsy_causes_of_death audit_autopsy_causes_of_death; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_autopsy_causes_of_death AFTER INSERT OR DELETE OR UPDATE ON public.autopsy_causes_of_death FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: case_e2b_field_notations audit_case_e2b_field_notations; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_case_e2b_field_notations AFTER INSERT OR DELETE OR UPDATE ON public.case_e2b_field_notations FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: case_field_notations audit_case_field_notations; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_case_field_notations AFTER INSERT OR DELETE OR UPDATE ON public.case_field_notations FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: case_submissions audit_case_submissions; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_case_submissions AFTER INSERT OR DELETE OR UPDATE ON public.case_submissions FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: case_summary_information audit_case_summary_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_case_summary_information AFTER INSERT OR DELETE OR UPDATE ON public.case_summary_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: case_versions audit_case_versions; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_case_versions AFTER INSERT OR DELETE OR UPDATE ON public.case_versions FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: cases audit_cases; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_cases AFTER INSERT OR DELETE OR UPDATE ON public.cases FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: controlled_terminology_terms audit_controlled_terminology_terms; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_controlled_terminology_terms AFTER INSERT OR DELETE OR UPDATE ON public.controlled_terminology_terms FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_audit_id();


--
-- Name: documents_held_by_sender audit_documents_held_by_sender; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_documents_held_by_sender AFTER INSERT OR DELETE OR UPDATE ON public.documents_held_by_sender FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: dosage_information audit_dosage_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_dosage_information AFTER INSERT OR DELETE OR UPDATE ON public.dosage_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: drug_active_substances audit_drug_active_substances; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_drug_active_substances AFTER INSERT OR DELETE OR UPDATE ON public.drug_active_substances FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: drug_device_characteristics audit_drug_device_characteristics; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_drug_device_characteristics AFTER INSERT OR DELETE OR UPDATE ON public.drug_device_characteristics FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: drug_indications audit_drug_indications; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_drug_indications AFTER INSERT OR DELETE OR UPDATE ON public.drug_indications FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: drug_information audit_drug_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_drug_information AFTER INSERT OR DELETE OR UPDATE ON public.drug_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: drug_reaction_assessments audit_drug_reaction_assessments; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_drug_reaction_assessments AFTER INSERT OR DELETE OR UPDATE ON public.drug_reaction_assessments FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: e2b_code_lists audit_e2b_code_lists; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_e2b_code_lists AFTER INSERT OR DELETE OR UPDATE ON public.e2b_code_lists FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_audit_id();


--
-- Name: e_signatures audit_e_signatures; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_e_signatures AFTER INSERT OR DELETE OR UPDATE ON public.e_signatures FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: fda_device_codes audit_fda_device_codes; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_fda_device_codes AFTER INSERT OR DELETE OR UPDATE ON public.fda_device_codes FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: fda_device_information audit_fda_device_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_fda_device_information AFTER INSERT OR DELETE OR UPDATE ON public.fda_device_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: iso_countries audit_iso_countries; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_iso_countries AFTER INSERT OR DELETE OR UPDATE ON public.iso_countries FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_audit_id();


--
-- Name: linked_report_numbers audit_linked_report_numbers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_linked_report_numbers AFTER INSERT OR DELETE OR UPDATE ON public.linked_report_numbers FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: literature_references audit_literature_references; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_literature_references AFTER INSERT OR DELETE OR UPDATE ON public.literature_references FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: audit_logs audit_logs_lock_before_statement; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_logs_lock_before_statement BEFORE INSERT ON public.audit_logs FOR EACH STATEMENT EXECUTE FUNCTION public.lock_audit_log_append();


--
-- Name: audit_logs audit_logs_reject_truncate; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_logs_reject_truncate BEFORE TRUNCATE ON public.audit_logs FOR EACH STATEMENT EXECUTE FUNCTION public.prevent_audit_log_mutation();


--
-- Name: audit_logs audit_logs_reject_update_delete; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_logs_reject_update_delete BEFORE DELETE OR UPDATE ON public.audit_logs FOR EACH ROW EXECUTE FUNCTION public.prevent_audit_log_mutation();


--
-- Name: meddra_terms audit_meddra_terms; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_meddra_terms AFTER INSERT OR DELETE OR UPDATE ON public.meddra_terms FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_audit_id();


--
-- Name: medical_history_episodes audit_medical_history_episodes; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_medical_history_episodes AFTER INSERT OR DELETE OR UPDATE ON public.medical_history_episodes FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: mfds_product_substances audit_mfds_product_substances; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_mfds_product_substances AFTER INSERT OR DELETE OR UPDATE ON public.mfds_product_substances FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_audit_id();


--
-- Name: mfds_products audit_mfds_products; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_mfds_products AFTER INSERT OR DELETE OR UPDATE ON public.mfds_products FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_audit_id();


--
-- Name: narrative_information audit_narrative_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_narrative_information AFTER INSERT OR DELETE OR UPDATE ON public.narrative_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: narrative_presaves audit_narrative_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_narrative_presaves AFTER INSERT OR DELETE OR UPDATE ON public.narrative_presaves FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: organization_as2_profiles audit_organization_as2_profiles; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_organization_as2_profiles AFTER INSERT OR DELETE OR UPDATE ON public.organization_as2_profiles FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: organizations audit_organizations; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_organizations AFTER INSERT OR DELETE OR UPDATE ON public.organizations FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: other_case_identifiers audit_other_case_identifiers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_other_case_identifiers AFTER INSERT OR DELETE OR UPDATE ON public.other_case_identifiers FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: parent_information audit_parent_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_parent_information AFTER INSERT OR DELETE OR UPDATE ON public.parent_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: parent_medical_history audit_parent_medical_history; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_parent_medical_history AFTER INSERT OR DELETE OR UPDATE ON public.parent_medical_history FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: parent_past_drug_history audit_parent_past_drug_history; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_parent_past_drug_history AFTER INSERT OR DELETE OR UPDATE ON public.parent_past_drug_history FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: past_drug_history audit_past_drug_history; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_past_drug_history AFTER INSERT OR DELETE OR UPDATE ON public.past_drug_history FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: patient_death_information audit_patient_death_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_patient_death_information AFTER INSERT OR DELETE OR UPDATE ON public.patient_death_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: patient_identifiers audit_patient_identifiers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_patient_identifiers AFTER INSERT OR DELETE OR UPDATE ON public.patient_identifiers FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: patient_information audit_patient_info; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_patient_info AFTER INSERT OR DELETE OR UPDATE ON public.patient_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: permission_profiles audit_permission_profiles; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_permission_profiles AFTER INSERT OR DELETE OR UPDATE ON public.permission_profiles FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: primary_sources audit_primary_sources; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_primary_sources AFTER INSERT OR DELETE OR UPDATE ON public.primary_sources FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: product_presave_active_substances audit_product_presave_active_substances; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_product_presave_active_substances AFTER INSERT OR DELETE OR UPDATE ON public.product_presave_active_substances FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: product_presaves audit_product_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_product_presaves AFTER INSERT OR DELETE OR UPDATE ON public.product_presaves FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: reactions audit_reactions; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_reactions AFTER INSERT OR DELETE OR UPDATE ON public.reactions FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: receiver_presave_consignees audit_receiver_presave_consignees; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_receiver_presave_consignees AFTER INSERT OR DELETE OR UPDATE ON public.receiver_presave_consignees FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: receiver_presave_routes audit_receiver_presave_routes; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_receiver_presave_routes AFTER INSERT OR DELETE OR UPDATE ON public.receiver_presave_routes FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: receiver_presaves audit_receiver_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_receiver_presaves AFTER INSERT OR DELETE OR UPDATE ON public.receiver_presaves FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: relatedness_assessments audit_relatedness_assessments; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_relatedness_assessments AFTER INSERT OR DELETE OR UPDATE ON public.relatedness_assessments FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: reported_causes_of_death audit_reported_causes_of_death; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_reported_causes_of_death AFTER INSERT OR DELETE OR UPDATE ON public.reported_causes_of_death FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: reporter_presaves audit_reporter_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_reporter_presaves AFTER INSERT OR DELETE OR UPDATE ON public.reporter_presaves FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: safety_report_identification audit_safety_report_identification; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_safety_report_identification AFTER INSERT OR DELETE OR UPDATE ON public.safety_report_identification FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: sender_diagnoses audit_sender_diagnoses; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_sender_diagnoses AFTER INSERT OR DELETE OR UPDATE ON public.sender_diagnoses FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: sender_information audit_sender_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_sender_information AFTER INSERT OR DELETE OR UPDATE ON public.sender_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: sender_presave_gateways audit_sender_presave_gateways; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_sender_presave_gateways AFTER INSERT OR DELETE OR UPDATE ON public.sender_presave_gateways FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: sender_presave_responsible_persons audit_sender_presave_responsible_persons; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_sender_presave_responsible_persons AFTER INSERT OR DELETE OR UPDATE ON public.sender_presave_responsible_persons FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: sender_presaves audit_sender_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_sender_presaves AFTER INSERT OR DELETE OR UPDATE ON public.sender_presaves FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: source_documents audit_source_documents; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_source_documents AFTER INSERT OR DELETE OR UPDATE ON public.source_documents FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: study_fda_cross_reported_inds audit_study_fda_cross_reported_inds; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_study_fda_cross_reported_inds AFTER INSERT OR DELETE OR UPDATE ON public.study_fda_cross_reported_inds FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: study_information audit_study_information; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_study_information AFTER INSERT OR DELETE OR UPDATE ON public.study_information FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: study_presave_fda_cross_reported_ind_numbers audit_study_presave_fda_cross_reported_ind_numbers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_study_presave_fda_cross_reported_ind_numbers AFTER INSERT OR DELETE OR UPDATE ON public.study_presave_fda_cross_reported_ind_numbers FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: study_presave_products audit_study_presave_products; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_study_presave_products AFTER INSERT OR DELETE OR UPDATE ON public.study_presave_products FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: study_presave_registration_numbers audit_study_presave_registration_numbers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_study_presave_registration_numbers AFTER INSERT OR DELETE OR UPDATE ON public.study_presave_registration_numbers FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: study_presave_reporters audit_study_presave_reporters; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_study_presave_reporters AFTER INSERT OR DELETE OR UPDATE ON public.study_presave_reporters FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: study_presaves audit_study_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_study_presaves AFTER INSERT OR DELETE OR UPDATE ON public.study_presaves FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: study_registration_numbers audit_study_registration_numbers; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_study_registration_numbers AFTER INSERT OR DELETE OR UPDATE ON public.study_registration_numbers FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: submission_acks audit_submission_acks; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_submission_acks AFTER INSERT OR DELETE OR UPDATE ON public.submission_acks FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: submission_dispatch_state audit_submission_dispatch_state; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_submission_dispatch_state AFTER INSERT OR DELETE OR UPDATE ON public.submission_dispatch_state FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_submission_id();


--
-- Name: submission_events audit_submission_events; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_submission_events AFTER INSERT OR DELETE OR UPDATE ON public.submission_events FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: submission_idempotency audit_submission_idempotency; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_submission_idempotency AFTER INSERT OR DELETE OR UPDATE ON public.submission_idempotency FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: terminology_releases audit_terminology_releases; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_terminology_releases AFTER INSERT OR DELETE OR UPDATE ON public.terminology_releases FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_audit_id();


--
-- Name: test_results audit_test_results; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_test_results AFTER INSERT OR DELETE OR UPDATE ON public.test_results FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: users audit_users; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_users AFTER INSERT OR DELETE OR UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function();


--
-- Name: whodrug_products audit_whodrug_products; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER audit_whodrug_products AFTER INSERT OR DELETE OR UPDATE ON public.whodrug_products FOR EACH ROW EXECUTE FUNCTION public.audit_trigger_function_with_audit_id();


--
-- Name: authorization_audit_events authorization_audit_append_only; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authorization_audit_append_only BEFORE DELETE OR UPDATE ON public.authorization_audit_events FOR EACH ROW EXECUTE FUNCTION public.prevent_authorization_audit_mutation();


--
-- Name: authorization_roles authorization_roles_builtin_guard; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authorization_roles_builtin_guard BEFORE DELETE OR UPDATE ON public.authorization_roles FOR EACH ROW EXECUTE FUNCTION public.protect_builtin_authorization_role();


--
-- Name: organizations authz_guard_organization_role_compatibility; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_guard_organization_role_compatibility BEFORE UPDATE OF org_type ON public.organizations FOR EACH ROW EXECUTE FUNCTION public.authz_guard_organization_role_compatibility();


--
-- Name: user_role_assignments authz_increment_assignment_row_version; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_increment_assignment_row_version BEFORE UPDATE OF user_id, organization_id, role_id, active ON public.user_role_assignments FOR EACH ROW EXECUTE FUNCTION public.authz_increment_assignment_row_version();


--
-- Name: authorization_roles authz_increment_role_row_version; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_increment_role_row_version BEFORE UPDATE OF organization_id, identity_kind, active, built_in, deleted_at, role_class ON public.authorization_roles FOR EACH ROW EXECUTE FUNCTION public.authz_increment_role_row_version();


--
-- Name: user_organization_memberships authz_initialize_membership_revision; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_initialize_membership_revision AFTER INSERT ON public.user_organization_memberships FOR EACH ROW EXECUTE FUNCTION public.authz_initialize_membership_revision();


--
-- Name: organizations authz_initialize_organization_revision; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_initialize_organization_revision AFTER INSERT ON public.organizations FOR EACH ROW EXECUTE FUNCTION public.authz_initialize_organization_revision();


--
-- Name: authorization_roles authz_revision_organization_authorization_roles; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_organization_authorization_roles AFTER INSERT OR DELETE OR UPDATE OF organization_id, identity_kind, active, built_in, deleted_at, role_class, row_version ON public.authorization_roles FOR EACH ROW EXECUTE FUNCTION public.authz_revision_organization_authorization_roles();


--
-- Name: organizations authz_revision_organization_organizations; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_organization_organizations AFTER UPDATE OF active, org_type ON public.organizations FOR EACH ROW EXECUTE FUNCTION public.authz_revision_organization_organizations();


--
-- Name: product_presaves authz_revision_organization_product_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_organization_product_presaves AFTER INSERT OR DELETE OR UPDATE OF deleted, organization_id ON public.product_presaves FOR EACH ROW EXECUTE FUNCTION public.authz_revision_organization_scope_definition();


--
-- Name: role_grants authz_revision_organization_role_grants; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_organization_role_grants BEFORE INSERT OR DELETE OR UPDATE OF role_id, grant_id ON public.role_grants FOR EACH ROW EXECUTE FUNCTION public.authz_revision_organization_role_grants();


--
-- Name: sender_presaves authz_revision_organization_sender_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_organization_sender_presaves AFTER INSERT OR DELETE OR UPDATE OF deleted, organization_id ON public.sender_presaves FOR EACH ROW EXECUTE FUNCTION public.authz_revision_organization_scope_definition();


--
-- Name: study_presaves authz_revision_organization_study_presaves; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_organization_study_presaves AFTER INSERT OR DELETE OR UPDATE OF deleted, organization_id ON public.study_presaves FOR EACH ROW EXECUTE FUNCTION public.authz_revision_organization_scope_definition();


--
-- Name: user_organization_memberships authz_revision_principal_user_organization_memberships; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_principal_user_organization_memberships AFTER UPDATE OF user_id, active, organization_id ON public.user_organization_memberships FOR EACH ROW EXECUTE FUNCTION public.authz_revision_principal_user_organization_memberships();


--
-- Name: user_role_assignments authz_revision_principal_user_role_assignments; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_principal_user_role_assignments AFTER INSERT OR DELETE OR UPDATE OF user_id, organization_id, role_id, active, row_version ON public.user_role_assignments FOR EACH ROW EXECUTE FUNCTION public.authz_revision_principal_user_role_assignments();


--
-- Name: users authz_revision_principal_users; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER authz_revision_principal_users AFTER UPDATE OF active, access_start_at, access_end_at, access_sender_ids, access_product_ids, access_study_ids, access_blind_allowed, active_sender_identifier ON public.users FOR EACH ROW EXECUTE FUNCTION public.authz_revision_principal_users();


--
-- Name: drug_information guard_drug_information_source_product_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_drug_information_source_product_presave BEFORE INSERT OR UPDATE OF source_product_presave_id ON public.drug_information FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('product_presaves', 'source_product_presave_id', 'case');


--
-- Name: narrative_information guard_narrative_information_source_narrative_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_narrative_information_source_narrative_presave BEFORE INSERT OR UPDATE OF source_narrative_presave_id ON public.narrative_information FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('narrative_presaves', 'source_narrative_presave_id', 'case');


--
-- Name: primary_sources guard_primary_sources_source_reporter_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_primary_sources_source_reporter_presave BEFORE INSERT OR UPDATE OF source_reporter_presave_id ON public.primary_sources FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('reporter_presaves', 'source_reporter_presave_id', 'case');


--
-- Name: product_presaves guard_product_presaves_receiver_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_product_presaves_receiver_presave BEFORE INSERT OR UPDATE OF receiver_presave_id ON public.product_presaves FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('receiver_presaves', 'receiver_presave_id', 'organization');


--
-- Name: product_presaves guard_product_presaves_sender_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_product_presaves_sender_presave BEFORE INSERT OR UPDATE OF sender_presave_id ON public.product_presaves FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('sender_presaves', 'sender_presave_id', 'organization');


--
-- Name: sender_information guard_sender_information_source_sender_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_sender_information_source_sender_presave BEFORE INSERT OR UPDATE OF source_sender_presave_id ON public.sender_information FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('sender_presaves', 'source_sender_presave_id', 'case');


--
-- Name: study_information guard_study_information_source_study_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_study_information_source_study_presave BEFORE INSERT OR UPDATE OF source_study_presave_id ON public.study_information FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('study_presaves', 'source_study_presave_id', 'case');


--
-- Name: study_presave_products guard_study_presave_products_product_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_study_presave_products_product_presave BEFORE INSERT OR UPDATE OF product_presave_id ON public.study_presave_products FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('product_presaves', 'product_presave_id', 'study');


--
-- Name: study_presave_reporters guard_study_presave_reporters_reporter_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_study_presave_reporters_reporter_presave BEFORE INSERT OR UPDATE OF reporter_presave_id ON public.study_presave_reporters FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('reporter_presaves', 'reporter_presave_id', 'study');


--
-- Name: study_presaves guard_study_presaves_product_presave; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER guard_study_presaves_product_presave BEFORE INSERT OR UPDATE OF product_presave_id ON public.study_presaves FOR EACH ROW EXECUTE FUNCTION public.require_active_presave_reference('product_presaves', 'product_presave_id', 'organization');


--
-- Name: case_submissions protect_submission_artifact; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER protect_submission_artifact BEFORE UPDATE ON public.case_submissions FOR EACH ROW EXECUTE FUNCTION public.protect_submission_artifact();


--
-- Name: role_grants role_grants_assignment_guard; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER role_grants_assignment_guard BEFORE INSERT OR UPDATE ON public.role_grants FOR EACH ROW EXECUTE FUNCTION public.enforce_role_grant_assignment();


--
-- Name: role_grants role_grants_builtin_guard; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER role_grants_builtin_guard BEFORE INSERT OR DELETE OR UPDATE ON public.role_grants FOR EACH ROW EXECUTE FUNCTION public.protect_builtin_role_grant();


--
-- Name: audit_logs trg_audit_logs_hash_chain_before_insert; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_audit_logs_hash_chain_before_insert BEFORE INSERT ON public.audit_logs FOR EACH ROW EXECUTE FUNCTION public.audit_logs_hash_chain_before_insert();


--
-- Name: dosage_information trg_nfv_dosage_information_52a6bc6d6b; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_dosage_information_52a6bc6d6b BEFORE INSERT OR UPDATE ON public.dosage_information FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('dose_form', 'dose_form_null_flavor', 'first_administration_date', 'first_administration_date_null_flavor', 'last_administration_date', 'last_administration_date_null_flavor', 'parent_route', 'parent_route_null_flavor', 'route_of_administration', 'route_of_administration_null_flavor');


--
-- Name: drug_indications trg_nfv_drug_indications_18ec116774; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_drug_indications_18ec116774 BEFORE INSERT OR UPDATE ON public.drug_indications FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('indication_text', 'indication_text_null_flavor');


--
-- Name: drug_information trg_nfv_drug_information_4ae36371f8; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_drug_information_4ae36371f8 BEFORE INSERT OR UPDATE ON public.drug_information FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('fda_additional_info_coded', 'fda_additional_info_coded_null_flavor');


--
-- Name: fda_device_information trg_nfv_fda_device_information_025d254d3f; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_fda_device_information_025d254d3f BEFORE INSERT OR UPDATE ON public.fda_device_information FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('common_device_name', 'common_device_name_null_flavor', 'device_brand_name', 'device_brand_name_null_flavor');


--
-- Name: literature_references trg_nfv_literature_references_c7c69bd474; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_literature_references_c7c69bd474 BEFORE INSERT OR UPDATE ON public.literature_references FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('reference_text', 'reference_text_null_flavor');


--
-- Name: medical_history_episodes trg_nfv_medical_history_episodes_2beeb3d331; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_medical_history_episodes_2beeb3d331 BEFORE INSERT OR UPDATE ON public.medical_history_episodes FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('continuing', 'continuing_null_flavor', 'end_date', 'end_date_null_flavor', 'start_date', 'start_date_null_flavor');


--
-- Name: parent_information trg_nfv_parent_information_62b167be8d; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_parent_information_62b167be8d BEFORE INSERT OR UPDATE ON public.parent_information FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('last_menstrual_period_date', 'last_menstrual_period_date_null_flavor', 'parent_birth_date', 'parent_birth_date_null_flavor', 'parent_identification', 'parent_identification_null_flavor', 'sex', 'sex_null_flavor');


--
-- Name: parent_medical_history trg_nfv_parent_medical_history_7432e61317; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_parent_medical_history_7432e61317 BEFORE INSERT OR UPDATE ON public.parent_medical_history FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('continuing', 'continuing_null_flavor', 'end_date', 'end_date_null_flavor', 'start_date', 'start_date_null_flavor');


--
-- Name: parent_past_drug_history trg_nfv_parent_past_drug_history_5f455232bd; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_parent_past_drug_history_5f455232bd BEFORE INSERT OR UPDATE ON public.parent_past_drug_history FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('end_date', 'end_date_null_flavor', 'start_date', 'start_date_null_flavor');


--
-- Name: past_drug_history trg_nfv_past_drug_history_4c5b12bd4a; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_past_drug_history_4c5b12bd4a BEFORE INSERT OR UPDATE ON public.past_drug_history FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('drug_name', 'drug_name_null_flavor', 'end_date', 'end_date_null_flavor', 'start_date', 'start_date_null_flavor');


--
-- Name: patient_death_information trg_nfv_patient_death_informatio_4709497da5; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_patient_death_informatio_4709497da5 BEFORE INSERT OR UPDATE ON public.patient_death_information FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('autopsy_performed', 'autopsy_performed_null_flavor', 'date_of_death', 'date_of_death_null_flavor');


--
-- Name: patient_identifiers trg_nfv_patient_identifiers_449a05aa50; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_patient_identifiers_449a05aa50 BEFORE INSERT OR UPDATE ON public.patient_identifiers FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('identifier_value', 'identifier_value_null_flavor');


--
-- Name: patient_information trg_nfv_patient_information_705c7159d6; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_patient_information_705c7159d6 BEFORE INSERT OR UPDATE ON public.patient_information FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('birth_date', 'birth_date_null_flavor', 'ethnicity_code', 'ethnicity_code_null_flavor', 'last_menstrual_period_date', 'last_menstrual_period_date_null_flavor', 'medical_history_text', 'medical_history_text_null_flavor', 'patient_initials', 'patient_initials_null_flavor', 'sex', 'sex_null_flavor');


--
-- Name: primary_sources trg_nfv_primary_sources_bd15abfa2c; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_primary_sources_bd15abfa2c BEFORE INSERT OR UPDATE ON public.primary_sources FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('city', 'city_null_flavor', 'department', 'department_null_flavor', 'email', 'email_null_flavor', 'organization', 'organization_null_flavor', 'postcode', 'postcode_null_flavor', 'qualification', 'qualification_null_flavor', 'reporter_family_name', 'reporter_family_name_null_flavor', 'reporter_given_name', 'reporter_given_name_null_flavor', 'reporter_middle_name', 'reporter_middle_name_null_flavor', 'reporter_title', 'reporter_title_null_flavor', 'state', 'state_null_flavor', 'street', 'street_null_flavor', 'telephone', 'telephone_null_flavor');


--
-- Name: reactions trg_nfv_reactions_3619f4f257; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_reactions_3619f4f257 BEFORE INSERT OR UPDATE ON public.reactions FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('criteria_congenital_anomaly', 'criteria_congenital_anomaly_null_flavor', 'criteria_death', 'criteria_death_null_flavor', 'criteria_disabling', 'criteria_disabling_null_flavor', 'criteria_hospitalization', 'criteria_hospitalization_null_flavor', 'criteria_life_threatening', 'criteria_life_threatening_null_flavor', 'criteria_other_medically_important', 'criteria_other_medically_important_null_flavor', 'end_date', 'end_date_null_flavor', 'required_intervention', 'required_intervention_null_flavor', 'start_date', 'start_date_null_flavor');


--
-- Name: relatedness_assessments trg_nfv_relatedness_assessments_d0568d1285; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_relatedness_assessments_d0568d1285 BEFORE INSERT OR UPDATE ON public.relatedness_assessments FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('result_of_assessment_kr1', 'result_of_assessment_kr1_null_flavor');


--
-- Name: reporter_presaves trg_nfv_reporter_presaves_c8caa3c3a8; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_reporter_presaves_c8caa3c3a8 BEFORE INSERT OR UPDATE ON public.reporter_presaves FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('city', 'city_null_flavor', 'department', 'department_null_flavor', 'organization', 'organization_null_flavor', 'postcode', 'postcode_null_flavor', 'qualification', 'qualification_null_flavor', 'reporter_email', 'reporter_email_null_flavor', 'reporter_family_name', 'reporter_family_name_null_flavor', 'reporter_given_name', 'reporter_given_name_null_flavor', 'reporter_middle_name', 'reporter_middle_name_null_flavor', 'reporter_title', 'reporter_title_null_flavor', 'state', 'state_null_flavor', 'street', 'street_null_flavor', 'telephone', 'telephone_null_flavor');


--
-- Name: safety_report_identification trg_nfv_safety_report_identifica_17a664c168; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_safety_report_identifica_17a664c168 BEFORE INSERT OR UPDATE ON public.safety_report_identification FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('combination_product_report_indicator', 'combination_product_report_indicator_null_flavor', 'fulfil_expedited_criteria', 'fulfil_expedited_criteria_null_flavor', 'other_case_identifiers_exist', 'other_case_identifiers_exist_null_flavor');


--
-- Name: sender_information trg_nfv_sender_information_80b7f40c46; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_sender_information_80b7f40c46 BEFORE INSERT OR UPDATE ON public.sender_information FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('person_family_name', 'person_family_name_null_flavor', 'person_given_name', 'person_given_name_null_flavor', 'person_middle_name', 'person_middle_name_null_flavor', 'person_title', 'person_title_null_flavor');


--
-- Name: study_fda_cross_reported_inds trg_nfv_study_fda_cross_reported_ec60e477aa; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_study_fda_cross_reported_ec60e477aa BEFORE INSERT OR UPDATE ON public.study_fda_cross_reported_inds FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('ind_number', 'ind_number_null_flavor');


--
-- Name: study_information trg_nfv_study_information_8c04b06ead; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_study_information_8c04b06ead BEFORE INSERT OR UPDATE ON public.study_information FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('sponsor_study_number', 'sponsor_study_number_null_flavor', 'study_name', 'study_name_null_flavor');


--
-- Name: study_presave_fda_cross_reported_ind_numbers trg_nfv_study_presave_fda_cross__eb0a190203; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_study_presave_fda_cross__eb0a190203 BEFORE INSERT OR UPDATE ON public.study_presave_fda_cross_reported_ind_numbers FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('ind_number', 'ind_number_null_flavor');


--
-- Name: study_presave_registration_numbers trg_nfv_study_presave_registrati_b3a4ad37bc; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_study_presave_registrati_b3a4ad37bc BEFORE INSERT OR UPDATE ON public.study_presave_registration_numbers FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('country_code', 'country_code_null_flavor', 'registration_number', 'registration_number_null_flavor');


--
-- Name: study_presaves trg_nfv_study_presaves_e35860b345; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_study_presaves_e35860b345 BEFORE INSERT OR UPDATE ON public.study_presaves FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('sponsor_study_number', 'sponsor_study_number_null_flavor', 'study_name', 'study_name_null_flavor');


--
-- Name: study_registration_numbers trg_nfv_study_registration_numbe_9fe791980a; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_study_registration_numbe_9fe791980a BEFORE INSERT OR UPDATE ON public.study_registration_numbers FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('country_code', 'country_code_null_flavor', 'registration_number', 'registration_number_null_flavor');


--
-- Name: test_results trg_nfv_test_results_0b13e32562; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_nfv_test_results_0b13e32562 BEFORE INSERT OR UPDATE ON public.test_results FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('test_date', 'test_date_null_flavor');


--
-- Name: parent_medical_history trg_parent_medical_history_continuing_nfv; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_parent_medical_history_continuing_nfv BEFORE INSERT OR UPDATE ON public.parent_medical_history FOR EACH ROW EXECUTE FUNCTION public.enforce_value_null_flavor_exclusivity('continuing', 'continuing_null_flavor');


--
-- Name: reactions trg_reaction_serious_derived; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trg_reaction_serious_derived BEFORE INSERT OR UPDATE OF serious, term_highlighted, criteria_death, criteria_life_threatening, criteria_hospitalization, criteria_disabling, criteria_congenital_anomaly, criteria_other_medically_important ON public.reactions FOR EACH ROW EXECUTE FUNCTION public.set_reaction_serious_derived();


--
-- Name: autopsy_causes_of_death update_autopsy_causes_of_death_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_autopsy_causes_of_death_updated_at BEFORE UPDATE ON public.autopsy_causes_of_death FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: case_e2b_field_notations update_case_e2b_field_notations_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_case_e2b_field_notations_updated_at BEFORE UPDATE ON public.case_e2b_field_notations FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: case_field_notations update_case_field_notations_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_case_field_notations_updated_at BEFORE UPDATE ON public.case_field_notations FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: case_submissions update_case_submissions_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_case_submissions_updated_at BEFORE UPDATE ON public.case_submissions FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: case_summary_information update_case_summary_information_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_case_summary_information_updated_at BEFORE UPDATE ON public.case_summary_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: cases update_cases_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_cases_updated_at BEFORE UPDATE ON public.cases FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: documents_held_by_sender update_documents_held_by_sender_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_documents_held_by_sender_updated_at BEFORE UPDATE ON public.documents_held_by_sender FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: dosage_information update_dosage_information_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_dosage_information_updated_at BEFORE UPDATE ON public.dosage_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: drug_active_substances update_drug_active_substances_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_drug_active_substances_updated_at BEFORE UPDATE ON public.drug_active_substances FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: drug_device_characteristics update_drug_device_characteristics_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_drug_device_characteristics_updated_at BEFORE UPDATE ON public.drug_device_characteristics FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: drug_indications update_drug_indications_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_drug_indications_updated_at BEFORE UPDATE ON public.drug_indications FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: drug_information update_drug_info_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_drug_info_updated_at BEFORE UPDATE ON public.drug_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: drug_reaction_assessments update_drug_reaction_assessments_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_drug_reaction_assessments_updated_at BEFORE UPDATE ON public.drug_reaction_assessments FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: e_signatures update_e_signatures_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_e_signatures_updated_at BEFORE UPDATE ON public.e_signatures FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: linked_report_numbers update_linked_report_numbers_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_linked_report_numbers_updated_at BEFORE UPDATE ON public.linked_report_numbers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: literature_references update_literature_references_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_literature_references_updated_at BEFORE UPDATE ON public.literature_references FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: medical_history_episodes update_medical_history_episodes_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_medical_history_episodes_updated_at BEFORE UPDATE ON public.medical_history_episodes FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: narrative_information update_narrative_information_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_narrative_information_updated_at BEFORE UPDATE ON public.narrative_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: narrative_presaves update_narrative_presaves_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_narrative_presaves_updated_at BEFORE UPDATE ON public.narrative_presaves FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: organization_as2_profiles update_organization_as2_profiles_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_organization_as2_profiles_updated_at BEFORE UPDATE ON public.organization_as2_profiles FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: organizations update_organizations_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_organizations_updated_at BEFORE UPDATE ON public.organizations FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: other_case_identifiers update_other_case_identifiers_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_other_case_identifiers_updated_at BEFORE UPDATE ON public.other_case_identifiers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: parent_information update_parent_information_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_parent_information_updated_at BEFORE UPDATE ON public.parent_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: parent_medical_history update_parent_medical_history_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_parent_medical_history_updated_at BEFORE UPDATE ON public.parent_medical_history FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: parent_past_drug_history update_parent_past_drug_history_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_parent_past_drug_history_updated_at BEFORE UPDATE ON public.parent_past_drug_history FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: past_drug_history update_past_drug_history_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_past_drug_history_updated_at BEFORE UPDATE ON public.past_drug_history FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: patient_death_information update_patient_death_info_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_patient_death_info_updated_at BEFORE UPDATE ON public.patient_death_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: patient_identifiers update_patient_identifiers_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_patient_identifiers_updated_at BEFORE UPDATE ON public.patient_identifiers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: patient_information update_patient_info_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_patient_info_updated_at BEFORE UPDATE ON public.patient_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: primary_sources update_primary_sources_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_primary_sources_updated_at BEFORE UPDATE ON public.primary_sources FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: product_presave_active_substances update_product_presave_active_substances_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_product_presave_active_substances_updated_at BEFORE UPDATE ON public.product_presave_active_substances FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: product_presaves update_product_presaves_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_product_presaves_updated_at BEFORE UPDATE ON public.product_presaves FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: reactions update_reactions_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_reactions_updated_at BEFORE UPDATE ON public.reactions FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: receiver_presave_consignees update_receiver_presave_consignees_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_receiver_presave_consignees_updated_at BEFORE UPDATE ON public.receiver_presave_consignees FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: receiver_presave_routes update_receiver_presave_routes_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_receiver_presave_routes_updated_at BEFORE UPDATE ON public.receiver_presave_routes FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: receiver_presaves update_receiver_presaves_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_receiver_presaves_updated_at BEFORE UPDATE ON public.receiver_presaves FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: relatedness_assessments update_relatedness_assessments_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_relatedness_assessments_updated_at BEFORE UPDATE ON public.relatedness_assessments FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: reported_causes_of_death update_reported_causes_of_death_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_reported_causes_of_death_updated_at BEFORE UPDATE ON public.reported_causes_of_death FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: reporter_presaves update_reporter_presaves_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_reporter_presaves_updated_at BEFORE UPDATE ON public.reporter_presaves FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: safety_report_identification update_safety_report_identification_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_safety_report_identification_updated_at BEFORE UPDATE ON public.safety_report_identification FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: sender_diagnoses update_sender_diagnoses_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_sender_diagnoses_updated_at BEFORE UPDATE ON public.sender_diagnoses FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: sender_information update_sender_information_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_sender_information_updated_at BEFORE UPDATE ON public.sender_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: sender_presave_gateways update_sender_presave_gateways_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_sender_presave_gateways_updated_at BEFORE UPDATE ON public.sender_presave_gateways FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: sender_presave_responsible_persons update_sender_presave_responsible_persons_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_sender_presave_responsible_persons_updated_at BEFORE UPDATE ON public.sender_presave_responsible_persons FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: sender_presaves update_sender_presaves_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_sender_presaves_updated_at BEFORE UPDATE ON public.sender_presaves FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: study_fda_cross_reported_inds update_study_fda_cross_reported_inds_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_study_fda_cross_reported_inds_updated_at BEFORE UPDATE ON public.study_fda_cross_reported_inds FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: study_information update_study_information_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_study_information_updated_at BEFORE UPDATE ON public.study_information FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: study_presave_fda_cross_reported_ind_numbers update_study_presave_fda_cross_reported_ind_numbers_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_study_presave_fda_cross_reported_ind_numbers_updated_at BEFORE UPDATE ON public.study_presave_fda_cross_reported_ind_numbers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: study_presave_products update_study_presave_products_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_study_presave_products_updated_at BEFORE UPDATE ON public.study_presave_products FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: study_presave_registration_numbers update_study_presave_registration_numbers_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_study_presave_registration_numbers_updated_at BEFORE UPDATE ON public.study_presave_registration_numbers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: study_presave_reporters update_study_presave_reporters_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_study_presave_reporters_updated_at BEFORE UPDATE ON public.study_presave_reporters FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: study_presaves update_study_presaves_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_study_presaves_updated_at BEFORE UPDATE ON public.study_presaves FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: study_registration_numbers update_study_registration_numbers_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_study_registration_numbers_updated_at BEFORE UPDATE ON public.study_registration_numbers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: submission_dispatch_state update_submission_dispatch_state_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_submission_dispatch_state_updated_at BEFORE UPDATE ON public.submission_dispatch_state FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: test_results update_test_results_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_test_results_updated_at BEFORE UPDATE ON public.test_results FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: users update_users_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: user_role_assignments user_role_assignments_scope_guard; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER user_role_assignments_scope_guard BEFORE INSERT OR UPDATE ON public.user_role_assignments FOR EACH ROW EXECUTE FUNCTION public.enforce_user_role_assignment_scope();
