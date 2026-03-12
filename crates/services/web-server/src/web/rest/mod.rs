// Declare handler modules
pub mod case_rest;
pub mod case_validation_rest;
pub mod compliance;
pub mod organization_rest;
pub mod patient_rest;
pub mod user_rest;

pub mod drug_rest;
pub mod message_header_rest;
pub mod narrative_rest;
pub mod reaction_rest;
pub mod safety_report_rest;
pub mod test_result_rest;

// Newly enabled modules
pub mod admin_role_rest;
pub mod admin_settings_rest;
pub mod audit_rest;
pub mod case_identifiers_rest;
pub mod drug_reaction_assessment_rest;
pub mod drug_recurrence_rest;
pub mod drug_sub_rest;
pub mod import_rest;
pub mod narrative_sub_rest;
pub mod parent_history_rest;
pub mod patient_sub_rest;
pub mod presave_template_rest;
pub mod receiver_rest;
pub mod relatedness_assessment_rest;
pub mod safety_report_sub_rest;
pub mod submission_rest;
pub mod terminology_rest;
pub mod validation_rules_rest;

use axum::routing::get;
use axum::Router;
use lib_core::model::ModelManager;
use lib_web::handlers::handlers_rest::rest_collection_item_routes;

/// Routes for /api/cases and nested subresources
pub fn routes_cases(mm: ModelManager) -> Router {
	rest_collection_item_routes(
		"/cases",
		"/cases/{id}",
		get(case_rest::list_cases).post(case_rest::create_case_guarded),
		get(case_rest::get_case)
			.put(case_rest::update_case_guarded)
			.delete(case_rest::delete_case),
	)
	.route(
		"/cases/intake-check",
		axum::routing::post(case_rest::check_case_intake_duplicate),
	)
	.route(
		"/cases/from-intake",
		axum::routing::post(case_rest::create_case_from_intake),
	)
	.route(
		"/cases/{id}/validator/mark-validated",
		axum::routing::post(case_rest::mark_case_validated_by_validator),
	)
	// Patient (singleton per case)
	.route(
		"/cases/{case_id}/patient",
		get(patient_rest::get_patient)
			.post(patient_rest::create_patient)
			.put(patient_rest::update_patient)
			.delete(patient_rest::delete_patient),
	)
	// Patient Identifiers (collection per patient) - D.1.1.x
	.route(
		"/cases/{case_id}/patient/identifiers",
		get(patient_sub_rest::list_patient_identifiers)
			.post(patient_sub_rest::create_patient_identifier),
	)
	.route(
		"/cases/{case_id}/patient/identifiers/{id}",
		get(patient_sub_rest::get_patient_identifier)
			.put(patient_sub_rest::update_patient_identifier)
			.delete(patient_sub_rest::delete_patient_identifier),
	)
	// Medical History Episodes (collection per patient) - D.7.1.r
	.route(
		"/cases/{case_id}/patient/medical-history",
		get(patient_sub_rest::list_medical_history_episodes)
			.post(patient_sub_rest::create_medical_history_episode),
	)
	.route(
		"/cases/{case_id}/patient/medical-history/{id}",
		get(patient_sub_rest::get_medical_history_episode)
			.put(patient_sub_rest::update_medical_history_episode)
			.delete(patient_sub_rest::delete_medical_history_episode),
	)
	// Past Drug History (collection per patient) - D.8.r
	.route(
		"/cases/{case_id}/patient/past-drugs",
		get(patient_sub_rest::list_past_drug_history)
			.post(patient_sub_rest::create_past_drug_history),
	)
	.route(
		"/cases/{case_id}/patient/past-drugs/{id}",
		get(patient_sub_rest::get_past_drug_history)
			.put(patient_sub_rest::update_past_drug_history)
			.delete(patient_sub_rest::delete_past_drug_history),
	)
	// Patient Death Information (collection per patient) - D.9
	.route(
		"/cases/{case_id}/patient/death-info",
		get(patient_sub_rest::list_patient_death_information)
			.post(patient_sub_rest::create_patient_death_information),
	)
	.route(
		"/cases/{case_id}/patient/death-info/{id}",
		get(patient_sub_rest::get_patient_death_information)
			.put(patient_sub_rest::update_patient_death_information)
			.delete(patient_sub_rest::delete_patient_death_information),
	)
	// Reported Causes of Death (collection per death info) - D.9.2.r
	.route(
		"/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes",
		get(patient_sub_rest::list_reported_causes_of_death)
			.post(patient_sub_rest::create_reported_cause_of_death),
	)
	.route(
		"/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}",
		get(patient_sub_rest::get_reported_cause_of_death)
			.put(patient_sub_rest::update_reported_cause_of_death)
			.delete(patient_sub_rest::delete_reported_cause_of_death),
	)
	// Autopsy Causes of Death (collection per death info) - D.9.4.r
	.route(
		"/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes",
		get(patient_sub_rest::list_autopsy_causes_of_death)
			.post(patient_sub_rest::create_autopsy_cause_of_death),
	)
	.route(
		"/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}",
		get(patient_sub_rest::get_autopsy_cause_of_death)
			.put(patient_sub_rest::update_autopsy_cause_of_death)
			.delete(patient_sub_rest::delete_autopsy_cause_of_death),
	)
	// Parent Information (collection per patient) - D.10
	.route(
		"/cases/{case_id}/patient/parents",
		get(patient_sub_rest::list_parent_information)
			.post(patient_sub_rest::create_parent_information),
	)
	.route(
		"/cases/{case_id}/patient/parents/{id}",
		get(patient_sub_rest::get_parent_information)
			.put(patient_sub_rest::update_parent_information)
			.delete(patient_sub_rest::delete_parent_information),
	)
	// Reactions (collection per case)
	.route(
		"/cases/{case_id}/reactions",
		get(reaction_rest::list_reactions).post(reaction_rest::create_reaction),
	)
	.route(
		"/cases/{case_id}/reactions/{id}",
		get(reaction_rest::get_reaction)
			.put(reaction_rest::update_reaction)
			.delete(reaction_rest::delete_reaction),
	)
	// Drugs (collection per case)
	.route(
		"/cases/{case_id}/drugs",
		get(drug_rest::list_drug_informations)
			.post(drug_rest::create_drug_information),
	)
	.route(
		"/cases/{case_id}/drugs/{id}",
		get(drug_rest::get_drug_information)
			.put(drug_rest::update_drug_information)
			.delete(drug_rest::delete_drug_information),
	)
	// Drug Active Substances (collection per drug) - G.k.2.3.r
	.route(
		"/cases/{case_id}/drugs/{drug_id}/active-substances",
		get(drug_sub_rest::list_drug_active_substances)
			.post(drug_sub_rest::create_drug_active_substance),
	)
	.route(
		"/cases/{case_id}/drugs/{drug_id}/active-substances/{id}",
		get(drug_sub_rest::get_drug_active_substance)
			.put(drug_sub_rest::update_drug_active_substance)
			.delete(drug_sub_rest::delete_drug_active_substance),
	)
	// Dosage Information (collection per drug) - G.k.4.r
	.route(
		"/cases/{case_id}/drugs/{drug_id}/dosages",
		get(drug_sub_rest::list_dosage_information)
			.post(drug_sub_rest::create_dosage_information),
	)
	.route(
		"/cases/{case_id}/drugs/{drug_id}/dosages/{id}",
		get(drug_sub_rest::get_dosage_information)
			.put(drug_sub_rest::update_dosage_information)
			.delete(drug_sub_rest::delete_dosage_information),
	)
	// Drug Indications (collection per drug) - G.k.6.r
	.route(
		"/cases/{case_id}/drugs/{drug_id}/indications",
		get(drug_sub_rest::list_drug_indications)
			.post(drug_sub_rest::create_drug_indication),
	)
	.route(
		"/cases/{case_id}/drugs/{drug_id}/indications/{id}",
		get(drug_sub_rest::get_drug_indication)
			.put(drug_sub_rest::update_drug_indication)
			.delete(drug_sub_rest::delete_drug_indication),
	)
	// Drug Device Characteristics (collection per drug) - FDA G.k.1.a / G.k.12.*
	.route(
		"/cases/{case_id}/drugs/{drug_id}/device-characteristics",
		get(drug_sub_rest::list_drug_device_characteristics)
			.post(drug_sub_rest::create_drug_device_characteristic),
	)
	.route(
		"/cases/{case_id}/drugs/{drug_id}/device-characteristics/{id}",
		get(drug_sub_rest::get_drug_device_characteristic)
			.put(drug_sub_rest::update_drug_device_characteristic)
			.delete(drug_sub_rest::delete_drug_device_characteristic),
	)
	// Drug-Reaction Assessments (collection per drug) - G.k.9.i
	.route(
		"/cases/{case_id}/drugs/{drug_id}/reaction-assessments",
		get(drug_reaction_assessment_rest::list_drug_reaction_assessments)
			.post(drug_reaction_assessment_rest::create_drug_reaction_assessment),
	)
	.route(
		"/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{id}",
		get(drug_reaction_assessment_rest::get_drug_reaction_assessment)
			.put(drug_reaction_assessment_rest::update_drug_reaction_assessment)
			.delete(drug_reaction_assessment_rest::delete_drug_reaction_assessment),
	)
	// Relatedness Assessments (collection per reaction assessment) - G.k.9.i.2.r
	.route(
		"/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness",
		get(relatedness_assessment_rest::list_relatedness_assessments)
			.post(relatedness_assessment_rest::create_relatedness_assessment),
	)
	.route(
		"/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}",
		get(relatedness_assessment_rest::get_relatedness_assessment)
			.put(relatedness_assessment_rest::update_relatedness_assessment)
			.delete(relatedness_assessment_rest::delete_relatedness_assessment),
	)
	// Drug Recurrences (collection per drug) - G.k.8.r
	.route(
		"/cases/{case_id}/drugs/{drug_id}/recurrences",
		get(drug_recurrence_rest::list_drug_recurrences)
			.post(drug_recurrence_rest::create_drug_recurrence),
	)
	.route(
		"/cases/{case_id}/drugs/{drug_id}/recurrences/{id}",
		get(drug_recurrence_rest::get_drug_recurrence)
			.put(drug_recurrence_rest::update_drug_recurrence)
			.delete(drug_recurrence_rest::delete_drug_recurrence),
	)
	// Test Results (collection per case)
	.route(
		"/cases/{case_id}/test-results",
		get(test_result_rest::list_test_results)
			.post(test_result_rest::create_test_result),
	)
	.route(
		"/cases/{case_id}/test-results/{id}",
		get(test_result_rest::get_test_result)
			.put(test_result_rest::update_test_result)
			.delete(test_result_rest::delete_test_result),
	)
	// Narrative (singleton per case)
	.route(
		"/cases/{case_id}/narrative",
		get(narrative_rest::get_narrative_information)
			.post(narrative_rest::create_narrative_information)
			.put(narrative_rest::update_narrative_information)
			.delete(narrative_rest::delete_narrative_information),
	)
	// Sender Diagnoses (collection per narrative) - H.3.r
	.route(
		"/cases/{case_id}/narrative/sender-diagnoses",
		get(narrative_sub_rest::list_sender_diagnoses)
			.post(narrative_sub_rest::create_sender_diagnosis),
	)
	.route(
		"/cases/{case_id}/narrative/sender-diagnoses/{id}",
		get(narrative_sub_rest::get_sender_diagnosis)
			.put(narrative_sub_rest::update_sender_diagnosis)
			.delete(narrative_sub_rest::delete_sender_diagnosis),
	)
	// Case Summary Information (collection per narrative) - H.5.r
	.route(
		"/cases/{case_id}/narrative/summaries",
		get(narrative_sub_rest::list_case_summary_information)
			.post(narrative_sub_rest::create_case_summary_information),
	)
	.route(
		"/cases/{case_id}/narrative/summaries/{id}",
		get(narrative_sub_rest::get_case_summary_information)
			.put(narrative_sub_rest::update_case_summary_information)
			.delete(narrative_sub_rest::delete_case_summary_information),
	)
	// Message Header (singleton per case)
	.route(
		"/cases/{case_id}/message-header",
		get(message_header_rest::get_message_header)
			.post(message_header_rest::create_message_header)
			.put(message_header_rest::update_message_header)
			.delete(message_header_rest::delete_message_header),
	)
	// Safety Report (singleton per case)
	.route(
		"/cases/{case_id}/safety-report",
		get(safety_report_rest::get_safety_report_identification)
			.post(safety_report_rest::create_safety_report_identification)
			.put(safety_report_rest::update_safety_report_identification)
			.delete(safety_report_rest::delete_safety_report_identification),
	)
	// Sender Information (collection per case) - C.3.x
	.route(
		"/cases/{case_id}/safety-report/senders",
		get(safety_report_sub_rest::list_sender_information)
			.post(safety_report_sub_rest::create_sender_information),
	)
	.route(
		"/cases/{case_id}/safety-report/senders/{id}",
		get(safety_report_sub_rest::get_sender_information)
			.put(safety_report_sub_rest::update_sender_information)
			.delete(safety_report_sub_rest::delete_sender_information),
	)
	// Primary Sources (collection per case) - C.2.r
	.route(
		"/cases/{case_id}/safety-report/primary-sources",
		get(safety_report_sub_rest::list_primary_sources)
			.post(safety_report_sub_rest::create_primary_source),
	)
	.route(
		"/cases/{case_id}/safety-report/primary-sources/{id}",
		get(safety_report_sub_rest::get_primary_source)
			.put(safety_report_sub_rest::update_primary_source)
			.delete(safety_report_sub_rest::delete_primary_source),
	)
	// Literature References (collection per case) - C.4.r
	.route(
		"/cases/{case_id}/safety-report/literature",
		get(safety_report_sub_rest::list_literature_references)
			.post(safety_report_sub_rest::create_literature_reference),
	)
	.route(
		"/cases/{case_id}/safety-report/literature/{id}",
		get(safety_report_sub_rest::get_literature_reference)
			.put(safety_report_sub_rest::update_literature_reference)
			.delete(safety_report_sub_rest::delete_literature_reference),
	)
	// Study Information (collection per case) - C.5
	.route(
		"/cases/{case_id}/safety-report/studies",
		get(safety_report_sub_rest::list_study_information)
			.post(safety_report_sub_rest::create_study_information),
	)
	.route(
		"/cases/{case_id}/safety-report/studies/{id}",
		get(safety_report_sub_rest::get_study_information)
			.put(safety_report_sub_rest::update_study_information)
			.delete(safety_report_sub_rest::delete_study_information),
	)
	// Study Registration Numbers (collection per study) - C.5.1.r
	.route(
		"/cases/{case_id}/safety-report/studies/{study_id}/registrations",
		get(safety_report_sub_rest::list_study_registration_numbers)
			.post(safety_report_sub_rest::create_study_registration_number),
	)
	.route(
		"/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}",
		get(safety_report_sub_rest::get_study_registration_number)
			.put(safety_report_sub_rest::update_study_registration_number)
			.delete(safety_report_sub_rest::delete_study_registration_number),
	)
	// Receiver (singleton per case) - Section A
	.route(
		"/cases/{case_id}/receiver",
		get(receiver_rest::get_receiver)
			.post(receiver_rest::create_receiver)
			.put(receiver_rest::update_receiver)
			.delete(receiver_rest::delete_receiver),
	)
	// Other Case Identifiers (collection per case) - C.1.9.r
	.route(
		"/cases/{case_id}/other-identifiers",
		get(case_identifiers_rest::list_other_case_identifiers)
			.post(case_identifiers_rest::create_other_case_identifier),
	)
	.route(
		"/cases/{case_id}/other-identifiers/{id}",
		get(case_identifiers_rest::get_other_case_identifier)
			.put(case_identifiers_rest::update_other_case_identifier)
			.delete(case_identifiers_rest::delete_other_case_identifier),
	)
	// Linked Report Numbers (collection per case) - C.1.10.r
	.route(
		"/cases/{case_id}/linked-reports",
		get(case_identifiers_rest::list_linked_report_numbers)
			.post(case_identifiers_rest::create_linked_report_number),
	)
	.route(
		"/cases/{case_id}/linked-reports/{id}",
		get(case_identifiers_rest::get_linked_report_number)
			.put(case_identifiers_rest::update_linked_report_number)
			.delete(case_identifiers_rest::delete_linked_report_number),
	)
	// Parent Medical History (collection per parent) - D.10.7.1.r
	.route(
		"/cases/{case_id}/patient/parent/{parent_id}/medical-history",
		get(parent_history_rest::list_parent_medical_history)
			.post(parent_history_rest::create_parent_medical_history),
	)
	.route(
		"/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}",
		get(parent_history_rest::get_parent_medical_history)
			.put(parent_history_rest::update_parent_medical_history)
			.delete(parent_history_rest::delete_parent_medical_history),
	)
	// Parent Past Drug History (collection per parent) - D.10.8.r
	.route(
		"/cases/{case_id}/patient/parent/{parent_id}/past-drugs",
		get(parent_history_rest::list_parent_past_drug_history)
			.post(parent_history_rest::create_parent_past_drug_history),
	)
	.route(
		"/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}",
		get(parent_history_rest::get_parent_past_drug_history)
			.put(parent_history_rest::update_parent_past_drug_history)
			.delete(parent_history_rest::delete_parent_past_drug_history),
	)
	// Case Versions (read-only collection per case)
	.route(
		"/cases/{case_id}/versions",
		get(audit_rest::list_case_versions),
	)
	.route(
		"/cases/{case_id}/validation",
		get(case_validation_rest::validate_case),
	)
	.route(
		"/cases/{case_id}/validation/all",
		get(case_validation_rest::validate_case_all),
	)
	.route("/exports/history", get(case_rest::list_xml_export_history))
	.route("/cases/link-options", get(case_rest::list_case_link_options))
	.route("/cases/{id}/export/xml", get(case_rest::export_case))
	.route("/cases/{id}/lifecycle", get(case_rest::get_case_lifecycle))
	.route(
		"/cases/{id}/submissions/fda",
		axum::routing::post(submission_rest::submit_case_to_fda),
	)
	.route(
		"/cases/{id}/submissions/mfds",
		axum::routing::post(submission_rest::submit_case_to_mfds),
	)
	.route(
		"/cases/{id}/submissions",
		get(submission_rest::list_case_submissions),
	)
	.with_state(mm)
}

/// Routes for /api/organizations
pub fn routes_organizations(mm: ModelManager) -> Router {
	rest_collection_item_routes(
		"/organizations",
		"/organizations/{id}",
		get(organization_rest::list_organizations)
			.post(organization_rest::create_organization),
		get(organization_rest::get_organization)
			.put(organization_rest::update_organization)
			.delete(organization_rest::delete_organization),
	)
	.with_state(mm)
}

/// Routes for /api/users
pub fn routes_users(mm: ModelManager) -> Router {
	Router::new()
		// GET /api/users/me - must be before /users/{id} to avoid matching
		.route("/users/me", get(user_rest::get_current_user))
		.route(
			"/users/me/password",
			axum::routing::post(user_rest::set_my_password),
		)
		// Standard collection routes
		.route(
			"/users",
			get(user_rest::list_users).post(user_rest::create_user),
		)
		.route(
			"/users/{id}",
			get(user_rest::get_user)
				.put(user_rest::update_user)
				.delete(user_rest::delete_user),
		)
		.route(
			"/admin/settings",
			get(admin_settings_rest::get_admin_settings)
				.put(admin_settings_rest::update_admin_settings),
		)
		.route(
			"/admin/roles",
			get(admin_role_rest::list_admin_roles)
				.post(admin_role_rest::create_admin_role),
		)
		.route(
			"/admin/roles/{role_name}",
			axum::routing::delete(admin_role_rest::delete_admin_role)
				.put(admin_role_rest::update_admin_role),
		)
		.with_state(mm)
}

/// Routes for /api/presave-templates
pub fn routes_presave_templates(mm: ModelManager) -> Router {
	Router::new()
		.route(
			"/presave-templates",
			get(presave_template_rest::list_presave_templates)
				.post(presave_template_rest::create_presave_template),
		)
		.route(
			"/presave-templates/{id}",
			get(presave_template_rest::get_presave_template)
				.patch(presave_template_rest::update_presave_template)
				.delete(presave_template_rest::delete_presave_template),
		)
		.route(
			"/presave-templates/{id}/audit",
			get(presave_template_rest::list_presave_template_audits),
		)
		.with_state(mm)
}

/// Routes for /api/terminology
pub fn routes_terminology(mm: ModelManager) -> Router {
	Router::new()
		.route("/terminology/meddra", get(terminology_rest::search_meddra))
		.route(
			"/terminology/whodrug",
			get(terminology_rest::search_whodrug),
		)
		.route(
			"/terminology/import/meddra",
			axum::routing::post(terminology_rest::import_meddra),
		)
		.route(
			"/terminology/import/whodrug",
			axum::routing::post(terminology_rest::import_whodrug),
		)
		.route(
			"/terminology/releases",
			get(terminology_rest::list_releases),
		)
		.route(
			"/terminology/releases/{dictionary}/{version}/approve",
			axum::routing::post(terminology_rest::approve_release),
		)
		.route(
			"/terminology/releases/{dictionary}/{version}/activate",
			axum::routing::post(terminology_rest::activate_release),
		)
		.route(
			"/terminology/releases/{dictionary}/{version}/rollback",
			axum::routing::post(terminology_rest::rollback_release),
		)
		.route(
			"/terminology/countries",
			get(terminology_rest::list_countries),
		)
		.route(
			"/terminology/code-lists",
			get(terminology_rest::get_code_list),
		)
		.route(
			"/terminology/ucum-units",
			get(terminology_rest::list_ucum_units),
		)
		.with_state(mm)
}

/// Routes for /api/import
pub fn routes_import(mm: ModelManager) -> Router {
	Router::new()
		.route("/import/xml/history", get(import_rest::list_import_history))
		.route(
			"/import/xml/validate",
			axum::routing::post(import_rest::validate_xml),
		)
		.route("/import/xml", axum::routing::post(import_rest::import_xml))
		.with_state(mm)
}

/// Routes for /api/audit-logs
pub fn routes_audit(mm: ModelManager) -> Router {
	Router::new()
		.route("/audit-logs", get(audit_rest::list_audit_logs))
		.route(
			"/audit-logs/verify-integrity",
			get(audit_rest::verify_audit_log_integrity),
		)
		.route(
			"/audit-logs/by-record/{table_name}/{record_id}",
			get(audit_rest::list_audit_logs_by_record),
		)
		.with_state(mm)
}

/// Routes for /api/validation
pub fn routes_validation(mm: ModelManager) -> Router {
	Router::new()
		.route(
			"/validation/rules",
			get(validation_rules_rest::list_validation_rules),
		)
		.with_state(mm)
}

/// Routes for /api/submissions
pub fn routes_submissions(mm: ModelManager) -> Router {
	Router::new()
		.route(
			"/submissions/history",
			get(submission_rest::list_all_submission_history),
		)
		.route(
			"/submissions/{id}",
			get(submission_rest::get_case_submission),
		)
		.route(
			"/submissions/{id}/events",
			get(submission_rest::list_submission_event_history),
		)
		.route(
			"/submissions/{id}/dispatch-state",
			get(submission_rest::get_submission_dispatch_state_view),
		)
		.route(
			"/submissions/{id}/acks/mock",
			axum::routing::post(submission_rest::post_mock_ack),
		)
		.with_state(mm)
}

/// Routes for /internal/submissions (gateway callbacks)
pub fn routes_submissions_internal(mm: ModelManager) -> Router {
	Router::new()
		.route(
			"/submissions/callbacks/ack",
			axum::routing::post(submission_rest::post_gateway_ack_callback),
		)
		.route(
			"/submissions/reconcile",
			axum::routing::post(submission_rest::post_reconcile_due_submissions),
		)
		.route(
			"/submissions/reconcile/status",
			get(submission_rest::get_reconcile_status),
		)
		.with_state(mm)
}
