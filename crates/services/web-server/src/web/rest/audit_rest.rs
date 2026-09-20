// Audit Log REST endpoints

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::audit::{
	AuditChainVerificationReport, AuditLog, AuditLogBmc, AuditLogFilter,
	CaseVersion, CaseVersionBmc,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::ParamsList;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	with_authorized_audit_log_collection, with_authorized_case_audit_read, Error,
	Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use modql::filter::OrderBy;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AuditRecordQuery {
	pub field: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseAuditTrailRow {
	pub no: i64,
	pub audit_log_id: i64,
	#[serde(with = "time::serde::rfc3339")]
	pub date_time: OffsetDateTime,
	pub user_display: Option<String>,
	pub page: String,
	pub item: String,
	pub row_no1: String,
	pub row_no2: String,
	pub row_no3: String,
	pub value: String,
	pub notation: String,
	pub null_flavor: String,
	pub reason: String,
	pub e_signature_id: Option<Uuid>,
}

fn json_object_has_key(value: &Option<JsonValue>, field: &str) -> bool {
	value
		.as_ref()
		.and_then(JsonValue::as_object)
		.is_some_and(|object| object.contains_key(field))
}

fn audit_log_touches_field(log: &AuditLog, field: &str) -> bool {
	if log.action == "UPDATE" {
		return json_object_has_key(&log.changed_fields, field);
	}
	json_object_has_key(&log.changed_fields, field)
		|| json_object_has_key(&log.old_values, field)
		|| json_object_has_key(&log.new_values, field)
}

fn audit_table_page(table_name: &str) -> Option<&'static str> {
	match table_name {
		"cases"
		| "safety_report_identification"
		| "documents_held_by_sender"
		| "other_case_identifiers"
		| "linked_report_numbers" => Some("CI (C.1)"),
		"primary_sources" => Some("RP (C.2.r)"),
		"sender_information" => Some("SD (C.3)"),
		"literature_references" => Some("LR (C.4.r)"),
		"study_information"
		| "study_registration_numbers"
		| "study_fda_cross_reported_inds" => Some("SI (C.5)"),
		"receiver_information" => Some("SD (A.1 Receiver)"),
		"patient_information"
		| "patient_identifiers"
		| "medical_history_episodes"
		| "patient_death_information"
		| "reported_causes_of_death"
		| "autopsy_causes_of_death"
		| "parent_information"
		| "parent_medical_history" => Some("DM (D)"),
		"past_drug_history" | "parent_past_drug_history" => Some("DH (D.8/D.10.8)"),
		"reactions" => Some("AE (E.i)"),
		"test_results" => Some("LB (F.r)"),
		"drug_information"
		| "drug_active_substances"
		| "dosage_information"
		| "drug_indications"
		| "drug_device_characteristics"
		| "fda_device_information"
		| "fda_device_codes"
		| "drug_reaction_assessments"
		| "relatedness_assessments" => Some("DG (G.k)"),
		"narrative_information"
		| "sender_diagnoses"
		| "case_summary_information" => Some("NR (H)"),
		"message_headers" => Some("SD (N)"),
		"case_versions" => Some("WF"),
		"case_workflow_events" => Some("WF"),
		"case_submissions"
		| "submission_events"
		| "submission_acks"
		| "submission_dispatch_state"
		| "submission_idempotency" => Some("RE"),
		"case_e2b_field_notations" | "case_field_notations" | "e_signatures" => {
			Some("AT")
		}
		_ => None,
	}
}

fn audit_business_field_label(
	table_name: &str,
	field: &str,
) -> Option<&'static str> {
	Some(match (table_name, field) {
		("cases", "mfds_report_type") => "MFDS Report Type (MFDS_REPORT_TYPE)",
		("cases", "fda_report_type") => "FDA Report Type (FDA_REPORT_TYPE)",
		("cases", "dg_prd_key") => "DG Product Key",
		("cases", "status") => "Case Status",
		("cases", "status_before_lock") => "Case Status Before Lock",
		("cases", "review_receivers_json") => "Review Receivers",
		("cases", "workflow_routes_json") => "Workflow Routes",
		("cases", "workflow_status") => "Workflow Status",
		("cases", "workflow_assigned_role") => "Workflow Assigned Role",
		("cases", "workflow_assigned_user_id") => "Workflow Assigned User",
		("cases", "workflow_due_at") => "Workflow Due Date",
		("cases", "workflow_description") => "Workflow Description",
		("cases", "workflow_updated_at") => "Workflow Updated At",
		("cases", "submitted_by") => "Submitted By",
		("cases", "submitted_at") => "Submitted At",
		("message_headers", "message_type") => "Message Type (N.1.1)",
		("message_headers", "batch_number") => "Batch Number (N.1.2)",
		("message_headers", "batch_sender_identifier") => {
			"Batch Sender Identifier (N.1.3)"
		}
		("message_headers", "batch_receiver_identifier") => {
			"Batch Receiver Identifier (N.1.4)"
		}
		("message_headers", "batch_transmission_date") => {
			"Date of Batch Transmission (N.1.5)"
		}
		("message_headers", "message_number") => "Message Number (N.2.r.1)",
		("message_headers", "message_sender_identifier") => {
			"Message Sender Identifier (N.2.r.2)"
		}
		("message_headers", "message_receiver_identifier") => {
			"Message Receiver Identifier (N.2.r.3)"
		}
		("message_headers", "message_date") => "Message Date (N.2.r.4)",
		("message_headers", "message_format_version") => "Message Format Version",
		("message_headers", "message_format_release") => "Message Format Release",
		("message_headers", "message_date_format") => "Message Date Format",
		("safety_report_identification", "safety_report_id") => {
			"Sender's Safety Report Unique Identifier (C.1.1)"
		}
		("safety_report_identification", "transmission_date") => {
			"Date of Creation (C.1.2)"
		}
		("safety_report_identification", "report_type") => "Type of Report (C.1.3)",
		("safety_report_identification", "version") => "Safety Report Version",
		("safety_report_identification", "local_criteria_report_type") => {
			"FDA Local Criteria Report Type (FDA.C.1.7.1)"
		}
		("safety_report_identification", "combination_product_report_indicator") => {
			"FDA Combination Product Report Indicator (FDA.C.1.12)"
		}
		("safety_report_identification", "receiver_organization") => {
			"Receiver Organization"
		}
		("safety_report_identification", "date_first_received_from_source") => {
			"Date First Received from Source (C.1.4)"
		}
		("safety_report_identification", "date_of_most_recent_information") => {
			"Date of Most Recent Information (C.1.5)"
		}
		("safety_report_identification", "additional_documents_available") => {
			"Additional Documents Available (C.1.6.1)"
		}
		("safety_report_identification", "fulfil_expedited_criteria") => {
			"Fulfils Expedited Criteria (C.1.7)"
		}
		("safety_report_identification", "worldwide_unique_id") => {
			"Worldwide Unique Case Identification (C.1.8.1)"
		}
		("safety_report_identification", "first_sender_type") => {
			"First Sender of This Case (C.1.8.2)"
		}
		("safety_report_identification", "other_case_identifiers_exist") => {
			"Other Case Identifiers Exist (C.1.9.1)"
		}
		("safety_report_identification", "nullification_code") => {
			"Nullification/Amendment Code (C.1.11.1)"
		}
		("safety_report_identification", "nullification_reason") => {
			"Nullification/Amendment Reason (C.1.11.2)"
		}
		("other_case_identifiers", "source_of_identifier") => {
			"Source of Case Identifier (C.1.9.1.r.1)"
		}
		("other_case_identifiers", "case_identifier") => {
			"Case Identifier (C.1.9.1.r.2)"
		}
		("linked_report_numbers", "linked_report_number") => {
			"Linked Report Number (C.1.10.r)"
		}
		("documents_held_by_sender", "title") => "Document Title (C.1.6.1.r.1)",
		("documents_held_by_sender", "document_base64") => {
			"Included Document (C.1.6.1.r.2)"
		}
		("documents_held_by_sender", "compression") => "Document Compression",
		("documents_held_by_sender", "file_name") => "Document File Name",
		("documents_held_by_sender", "media_type") => "Document Media Type",
		("documents_held_by_sender", "representation") => "Document Representation",
		("primary_sources", "reporter_title") => "Reporter's Title (C.2.r.1.1)",
		("primary_sources", "reporter_given_name") => {
			"Reporter's Given Name (C.2.r.1.2)"
		}
		("primary_sources", "reporter_middle_name") => {
			"Reporter's Middle Name (C.2.r.1.3)"
		}
		("primary_sources", "reporter_family_name") => {
			"Reporter's Family Name (C.2.r.1.4)"
		}
		("primary_sources", "organization") => "Reporter's Organisation (C.2.r.2.1)",
		("primary_sources", "department") => "Reporter's Department (C.2.r.2.2)",
		("primary_sources", "street") => "Reporter's Street (C.2.r.2.3)",
		("primary_sources", "city") => "Reporter's City (C.2.r.2.4)",
		("primary_sources", "state") => "Reporter's State or Province (C.2.r.2.5)",
		("primary_sources", "postcode") => "Reporter's Postcode (C.2.r.2.6)",
		("primary_sources", "telephone") => "Reporter's Telephone (C.2.r.2.7)",
		("primary_sources", "email") => "Reporter's Email (FDA.C.2.r.2.8)",
		("primary_sources", "country_code") => "Reporter's Country Code (C.2.r.3)",
		("primary_sources", "qualification") => "Reporter's Qualification (C.2.r.4)",
		("primary_sources", "qualification_kr1") => {
			"MFDS Reporter Qualification (C.2.r.4.KR.1)"
		}
		("primary_sources", "primary_source_regulatory") => {
			"Primary Source for Regulatory Purposes (C.2.r.5)"
		}
		("sender_information", "sender_type") => "Sender Type (C.3.1)",
		("sender_information", "health_professional_type_kr1") => {
			"Health Professional Type (C.3.1.KR.1)"
		}
		("sender_information", "organization_name") => {
			"Sender's Organisation (C.3.2)"
		}
		("sender_information", "department") => "Sender's Department (C.3.3.1)",
		("sender_information", "street_address") => {
			"Sender's Street Address (C.3.4.1)"
		}
		("sender_information", "city") => "Sender's City (C.3.4.2)",
		("sender_information", "state") => "Sender's State or Province (C.3.4.3)",
		("sender_information", "postcode") => "Sender's Postcode (C.3.4.4)",
		("sender_information", "country_code") => "Sender's Country Code (C.3.4.5)",
		("sender_information", "person_title") => "Sender's Title (C.3.3.2)",
		("sender_information", "person_given_name") => {
			"Sender's Given Name (C.3.3.3)"
		}
		("sender_information", "person_middle_name") => {
			"Sender's Middle Name (C.3.3.4)"
		}
		("sender_information", "person_family_name") => {
			"Sender's Family Name (C.3.3.5)"
		}
		("sender_information", "telephone") => "Sender's Telephone (C.3.4.6)",
		("sender_information", "fax") => "Sender's Fax (C.3.4.7)",
		("sender_information", "email") => "Sender's E-mail Address (C.3.4.8)",
		("receiver_information", "receiver_type") => "Receiver Type",
		("receiver_information", "organization_name") => "Receiver Organization",
		("receiver_information", "department") => "Receiver Department",
		("receiver_information", "street_address") => "Receiver Street Address",
		("receiver_information", "city") => "Receiver City",
		("receiver_information", "state_province") => "Receiver State or Province",
		("receiver_information", "postcode") => "Receiver Postcode",
		("receiver_information", "country_code") => "Receiver Country Code",
		("receiver_information", "telephone") => "Receiver Telephone",
		("receiver_information", "fax") => "Receiver Fax",
		("receiver_information", "email") => "Receiver Email",
		("literature_references", "reference_text") => {
			"Literature Reference (C.4.r.1)"
		}
		("literature_references", "document_base64") => {
			"Included Literature Document (C.4.r.2)"
		}
		("literature_references", "compression") => {
			"Literature Document Compression"
		}
		("literature_references", "file_name") => "Literature Document File Name",
		("literature_references", "media_type") => "Literature Document Media Type",
		("literature_references", "representation") => {
			"Literature Document Representation"
		}
		("study_information", "study_name") => "Study Name (C.5.2)",
		("study_information", "sponsor_study_number") => {
			"Sponsor Study Number (C.5.3)"
		}
		("study_information", "study_type_reaction") => {
			"Study Type in Which Reaction Was Observed (C.5.4)"
		}
		("study_information", "study_type_reaction_kr1") => {
			"MFDS Study Type in Which Reaction Was Observed (C.5.4.KR.1)"
		}
		("study_information", "fda_ind_number_occurred") => {
			"IND Number Where Reaction Occurred (FDA.C.5.5a)"
		}
		("study_information", "fda_pre_anda_number_occurred") => {
			"Pre-ANDA Number Where Reaction Occurred (FDA.C.5.5b)"
		}
		("study_registration_numbers", "registration_number") => {
			"Study Registration Number (C.5.1.r.1)"
		}
		("study_registration_numbers", "country_code") => {
			"Study Registration Country (C.5.1.r.2)"
		}
		("study_fda_cross_reported_inds", "ind_number") => {
			"Cross-reported IND Number (FDA.C.5)"
		}
		("patient_information", "patient_initials") => "Patient Initials (D.1)",
		("patient_information", "birth_date") => "Patient Birth Date (D.2.1)",
		("patient_information", "age_at_time_of_onset") => {
			"Age at Time of Reaction (D.2.2a)"
		}
		("patient_information", "age_unit") => "Age Unit (D.2.2b)",
		("patient_information", "gestation_period") => {
			"Gestation Period at Time of Reaction (D.2.2.1a)"
		}
		("patient_information", "gestation_period_unit") => {
			"Gestation Period Unit (D.2.2.1b)"
		}
		("patient_information", "age_group") => "Patient Age Group (D.2.3)",
		("patient_information", "weight_kg") => "Patient Weight (D.3)",
		("patient_information", "height_cm") => "Patient Height (D.4)",
		("patient_information", "sex") => "Sex (D.5)",
		("patient_information", "last_menstrual_period_date") => {
			"Last Menstrual Period Date (D.6)"
		}
		("patient_information", "medical_history_text") => {
			"Relevant Medical History and Concurrent Conditions (D.7.2)"
		}
		("patient_information", "concomitant_therapy") => {
			"Concomitant Therapies (D.7.3)"
		}
		("patient_information", "race_codes") => "Race (FDA.D.11.r.1)",
		("patient_information", "race_code_null_flavor") => {
			"Race (FDA.D.11.r.1) — Null Flavor"
		}
		("patient_information", "ethnicity_code") => "Ethnicity (FDA.D.12)",
		("patient_identifiers", "identifier_type_code") => {
			"Patient Record Number Type (D.1.1)"
		}
		("patient_identifiers", "identifier_value") => {
			"Patient Record Number (D.1.1)"
		}
		("medical_history_episodes", "meddra_version") => {
			"Medical History MedDRA Version (D.7.1.r.1a)"
		}
		("medical_history_episodes", "meddra_code") => {
			"Medical History (D.7.1.r.1b)"
		}
		("medical_history_episodes", "start_date") => {
			"Medical History Start Date (D.7.1.r.2)"
		}
		("medical_history_episodes", "continuing") => {
			"Medical History Continuing (D.7.1.r.3)"
		}
		("medical_history_episodes", "end_date") => {
			"Medical History End Date (D.7.1.r.4)"
		}
		("medical_history_episodes", "comments") => {
			"Medical History Comments (D.7.1.r.5)"
		}
		("medical_history_episodes", "family_history") => {
			"Family History (D.7.1.r.6)"
		}
		("patient_death_information", "date_of_death") => "Date of Death (D.9.1)",
		("patient_death_information", "autopsy_performed") => {
			"Autopsy Performed (D.9.3)"
		}
		("reported_causes_of_death", "meddra_version") => {
			"Reported Cause of Death MedDRA Version (D.9.2.r.1a)"
		}
		("reported_causes_of_death", "meddra_code") => {
			"Reported Cause of Death (D.9.2.r.1b)"
		}
		("reported_causes_of_death", "comments") => {
			"Reported Cause of Death Comments (D.9.2.r)"
		}
		("autopsy_causes_of_death", "meddra_version") => {
			"Autopsy Cause of Death MedDRA Version (D.9.4.r.1a)"
		}
		("autopsy_causes_of_death", "meddra_code") => {
			"Autopsy Cause of Death (D.9.4.r.1b)"
		}
		("autopsy_causes_of_death", "comments") => {
			"Autopsy Cause of Death Comments (D.9.4.r)"
		}
		("parent_information", "parent_identification") => {
			"Parent Identification (D.10.1)"
		}
		("parent_information", "parent_birth_date") => {
			"Parent Date of Birth (D.10.2.1)"
		}
		("parent_information", "parent_age") => "Parent Age (D.10.2.2a)",
		("parent_information", "parent_age_unit") => "Parent Age Unit (D.10.2.2b)",
		("parent_information", "last_menstrual_period_date") => {
			"Parent Last Menstrual Period Date (D.10.3)"
		}
		("parent_information", "weight_kg") => "Parent Weight (D.10.4)",
		("parent_information", "height_cm") => "Parent Height (D.10.5)",
		("parent_information", "sex") => "Parent Sex (D.10.6)",
		("parent_information", "medical_history_text") => {
			"Relevant Parent Medical History (D.10.7.2)"
		}
		("parent_medical_history", "meddra_version") => {
			"Parent Medical History MedDRA Version (D.10.7.1.r.1a)"
		}
		("parent_medical_history", "meddra_code") => {
			"Parent Medical History (D.10.7.1.r.1b)"
		}
		("parent_medical_history", "start_date") => {
			"Parent Medical History Start Date (D.10.7.1.r.2)"
		}
		("parent_medical_history", "continuing") => {
			"Parent Medical History Continuing (D.10.7.1.r.3)"
		}
		("parent_medical_history", "end_date") => {
			"Parent Medical History End Date (D.10.7.1.r.4)"
		}
		("parent_medical_history", "comments") => {
			"Parent Medical History Comments (D.10.7.1.r.5)"
		}
		("past_drug_history", "drug_name") => "Past Drug Name (D.8.r.1)",
		("past_drug_history", "mpid") => "Past Drug MPID (D.8.r.2b)",
		("past_drug_history", "mpid_version") => "Past Drug MPID Version (D.8.r.2a)",
		("past_drug_history", "mpid_source_code_system") => {
			"Past Drug MPID Source Code System"
		}
		("past_drug_history", "mpid_source_code_system_version") => {
			"Past Drug MPID Source Code System Version"
		}
		("past_drug_history", "phpid") => "Past Drug PhPID (D.8.r.3b)",
		("past_drug_history", "phpid_version") => {
			"Past Drug PhPID Version (D.8.r.3a)"
		}
		("past_drug_history", "start_date") => "Past Drug Start Date (D.8.r.4)",
		("past_drug_history", "end_date") => "Past Drug End Date (D.8.r.5)",
		("past_drug_history", "indication_meddra_version") => {
			"Past Drug Indication MedDRA Version (D.8.r.6a)"
		}
		("past_drug_history", "indication_meddra_code") => {
			"Past Drug Indication (D.8.r.6b)"
		}
		("past_drug_history", "reaction_meddra_version") => {
			"Past Drug Reaction MedDRA Version (D.8.r.7a)"
		}
		("past_drug_history", "reaction_meddra_code") => {
			"Past Drug Reaction (D.8.r.7b)"
		}
		("parent_past_drug_history", "drug_name") => {
			"Parent Past Drug Name (D.10.8.r.1)"
		}
		("parent_past_drug_history", "mpid_version") => {
			"Parent Past Drug MPID Version (D.10.8.r.2a)"
		}
		("parent_past_drug_history", "mpid") => {
			"Parent Past Drug MPID (D.10.8.r.2b)"
		}
		("parent_past_drug_history", "phpid_version") => {
			"Parent Past Drug PhPID Version (D.10.8.r.3a)"
		}
		("parent_past_drug_history", "phpid") => {
			"Parent Past Drug PhPID (D.10.8.r.3b)"
		}
		("parent_past_drug_history", "start_date") => {
			"Parent Past Drug Start Date (D.10.8.r.4)"
		}
		("parent_past_drug_history", "end_date") => {
			"Parent Past Drug End Date (D.10.8.r.5)"
		}
		("parent_past_drug_history", "indication_meddra_version") => {
			"Parent Past Drug Indication MedDRA Version (D.10.8.r.6a)"
		}
		("parent_past_drug_history", "indication_meddra_code") => {
			"Parent Past Drug Indication (D.10.8.r.6b)"
		}
		("parent_past_drug_history", "reaction_meddra_version") => {
			"Parent Past Drug Reaction MedDRA Version (D.10.8.r.7a)"
		}
		("parent_past_drug_history", "reaction_meddra_code") => {
			"Parent Past Drug Reaction (D.10.8.r.7b)"
		}
		("past_drug_history", "mfds_medicinal_product_version") => {
			"Past Drug MFDS Medicinal Product Version (D.8.r.1.KR.1a)"
		}
		("past_drug_history", "mfds_medicinal_product_id") => {
			"Past Drug MFDS Medicinal Product ID (D.8.r.1.KR.1b)"
		}
		("parent_past_drug_history", "mfds_medicinal_product_version") => {
			"Parent Past Drug MFDS Medicinal Product Version (D.10.8.r.1.KR.1a)"
		}
		("parent_past_drug_history", "mfds_medicinal_product_id") => {
			"Parent Past Drug MFDS Medicinal Product ID (D.10.8.r.1.KR.1b)"
		}
		("drug_information", "drug_characterization") => {
			"Characterization of Drug Role (G.k.1)"
		}
		("drug_information", "medicinal_product") => {
			"Medicinal Product Name as Reported (G.k.2.2)"
		}
		("drug_information", "mpid") => {
			"Medicinal Product Identifier (MPID) (G.k.2.1.1b)"
		}
		("drug_information", "mpid_version") => {
			"MPID Version Date / Number (G.k.2.1.1a)"
		}
		("drug_information", "mpid_source_code_system") => "MPID Source Code System",
		("drug_information", "mpid_source_code_system_version") => {
			"MPID Source Code System Version"
		}
		("drug_information", "phpid") => {
			"Pharmaceutical Product Identifier (PhPID) (G.k.2.1.2b)"
		}
		("drug_information", "phpid_version") => {
			"PhPID Version Date / Number (G.k.2.1.2a)"
		}
		("drug_information", "investigational_product_blinded") => {
			"Investigational Product Blinded (G.k.2.5)"
		}
		("drug_information", "obtain_drug_country") => {
			"Country Where Drug Was Obtained (G.k.2.4)"
		}
		("drug_information", "drug_authorization_number") => {
			"Drug Authorization Number (G.k.3.1)"
		}
		("drug_information", "manufacturer_name") => {
			"Authorization Holder Name (G.k.3.3)"
		}
		("drug_information", "manufacturer_country") => {
			"Authorization Country (G.k.3.2)"
		}
		("drug_information", "batch_lot_number") => "Batch/Lot Number",
		("drug_information", "cumulative_dose_first_reaction_value") => {
			"Cumulative Dose to First Reaction (G.k.5a)"
		}
		("drug_information", "cumulative_dose_first_reaction_unit") => {
			"Cumulative Dose Unit (G.k.5b)"
		}
		("drug_information", "gestation_period_exposure_value") => {
			"Gestation Period at Exposure (G.k.6a)"
		}
		("drug_information", "gestation_period_exposure_unit") => {
			"Gestation Period at Exposure Unit (G.k.6b)"
		}
		("drug_information", "action_taken") => "Action Taken with Drug (G.k.8)",
		("drug_information", "drug_additional_info_codes_json") => {
			"Additional Information on Drug, Coded"
		}
		("drug_information", "drug_additional_information") => {
			"Additional Information on Drug (G.k.11)"
		}
		("drug_information", "mfds_mpid_version") => {
			"MFDS Medicinal Product ID Version (G.k.2.1.KR.1a)"
		}
		("drug_information", "mfds_mpid") => {
			"MFDS Medicinal Product ID (G.k.2.1.KR.1b)"
		}
		("drug_information", "fda_additional_info_coded") => {
			"FDA Additional Information on Drug, Coded (FDA.G.k.10a)"
		}
		("drug_information", "fda_specialized_product_category") => {
			"FDA Specialized Product Category (FDA.G.k.10.1)"
		}
		("drug_information", "fda_other_characterization") => {
			"FDA Other Characterization of Drug Role (FDA.G.k.1.a)"
		}
		("drug_active_substances", "substance_name") => {
			"Substance Name (G.k.2.3.r.1)"
		}
		("drug_active_substances", "substance_termid") => {
			"Substance Term ID (G.k.2.3.r.2b)"
		}
		("drug_active_substances", "substance_termid_version") => {
			"Substance Term ID Version (G.k.2.3.r.2a)"
		}
		("drug_active_substances", "substance_termid_code_system") => {
			"Substance Term ID Code System"
		}
		("drug_active_substances", "strength_value") => {
			"Substance Strength (G.k.2.3.r.3a)"
		}
		("drug_active_substances", "strength_unit") => {
			"Substance Strength Unit (G.k.2.3.r.3b)"
		}
		("drug_active_substances", "mfds_version") => {
			"MFDS Substance Version (G.k.2.3.r.1.KR.1a)"
		}
		("drug_active_substances", "mfds_id") => {
			"MFDS Substance ID (G.k.2.3.r.1.KR.1b)"
		}
		("dosage_information", "dose_value") => "Dose (G.k.4.r.1a)",
		("dosage_information", "dose_unit") => "Dose Unit (G.k.4.r.1b)",
		("dosage_information", "number_of_units") => {
			"Number of Units in Interval (G.k.4.r.2)"
		}
		("dosage_information", "frequency_unit") => "Time Interval Unit (G.k.4.r.3)",
		("dosage_information", "first_administration_date") => {
			"Date of First Administration (G.k.4.r.4)"
		}
		("dosage_information", "first_administration_date_raw") => {
			"Date of First Administration — Source Lexical Value"
		}
		("dosage_information", "last_administration_date") => {
			"Date of Last Administration (G.k.4.r.5)"
		}
		("dosage_information", "last_administration_date_raw") => {
			"Date of Last Administration — Source Lexical Value"
		}
		("dosage_information", "duration_value") => {
			"Duration of Drug Administration (G.k.4.r.6a)"
		}
		("dosage_information", "duration_unit") => "Duration Unit (G.k.4.r.6b)",
		("dosage_information", "continuing") => "Drug Administration Continuing",
		("dosage_information", "batch_lot_number") => "Batch/Lot Number (G.k.4.r.7)",
		("dosage_information", "dosage_text") => "Dosage Text (G.k.4.r.8)",
		("dosage_information", "dose_form") => {
			"Pharmaceutical Dose Form (G.k.4.r.9.1)"
		}
		("dosage_information", "dose_form_termid_version") => {
			"Pharmaceutical Dose Form Term ID Version (G.k.4.r.9.2a)"
		}
		("dosage_information", "dose_form_termid") => {
			"Pharmaceutical Dose Form Term ID (G.k.4.r.9.2b)"
		}
		("dosage_information", "route_of_administration") => {
			"Route of Administration (G.k.4.r.10.1)"
		}
		("dosage_information", "route_termid_version") => {
			"Route Term ID Version (G.k.4.r.10.2a)"
		}
		("dosage_information", "route_termid") => "Route Term ID (G.k.4.r.10.2b)",
		("dosage_information", "route_termid_code_system") => {
			"Route Term ID Code System"
		}
		("dosage_information", "parent_route") => {
			"Parent Route of Administration (G.k.4.r.11.1)"
		}
		("dosage_information", "parent_route_termid_version") => {
			"Parent Route Term ID Version (G.k.4.r.11.2a)"
		}
		("dosage_information", "parent_route_termid") => {
			"Parent Route Term ID (G.k.4.r.11.2b)"
		}
		("dosage_information", "parent_route_termid_code_system") => {
			"Parent Route Term ID Code System"
		}
		("drug_indications", "indication_text") => {
			"Drug Indication as Reported (G.k.7.r.1)"
		}
		("drug_indications", "indication_meddra_version") => {
			"Drug Indication MedDRA Version (G.k.7.r.2a)"
		}
		("drug_indications", "indication_meddra_code") => {
			"Drug Indication (G.k.7.r.2b)"
		}
		("drug_reaction_assessments", "administration_start_interval_value") => {
			"Drug Start to Reaction Interval (G.k.9.i.3.1a)"
		}
		("drug_reaction_assessments", "administration_start_interval_unit") => {
			"Drug Start to Reaction Interval Unit (G.k.9.i.3.1b)"
		}
		("drug_reaction_assessments", "last_dose_interval_value") => {
			"Last Dose to Reaction Interval (G.k.9.i.3.2a)"
		}
		("drug_reaction_assessments", "last_dose_interval_unit") => {
			"Last Dose to Reaction Interval Unit (G.k.9.i.3.2b)"
		}
		("drug_reaction_assessments", "recurrence_action") => {
			"Readministration Action"
		}
		("drug_reaction_assessments", "reaction_recurred") => {
			"Reaction Recurred on Readministration (G.k.9.i.4.r.3)"
		}
		("drug_reaction_assessments", "dechallenge_result") => {
			"Reaction Abated After Stopping Drug (CIOMS Item 20)"
		}
		("drug_reaction_assessments", "expectedness") => {
			"Local Assessment Expectedness"
		}
		("relatedness_assessments", "source_of_assessment") => {
			"Source of Assessment (G.k.9.i.2.r.1)"
		}
		("relatedness_assessments", "method_of_assessment") => {
			"Method of Assessment (G.k.9.i.2.r.2)"
		}
		("relatedness_assessments", "result_of_assessment") => {
			"Result of Assessment (G.k.9.i.2.r.3)"
		}
		("relatedness_assessments", "method_of_assessment_kr1") => {
			"MFDS Method of Assessment (G.k.9.i.2.r.2.KR.1)"
		}
		("relatedness_assessments", "result_of_assessment_kr1") => {
			"MFDS Result of Assessment (G.k.9.i.2.r.3.KR.1)"
		}
		("relatedness_assessments", "result_of_assessment_kr2") => {
			"MFDS Additional Assessment Result (G.k.9.i.2.r.3.KR.2)"
		}
		("fda_device_information", "malfunction") => {
			"FDA Device Malfunction (FDA.G.k.12.r.1)"
		}
		("fda_device_information", "device_brand_name") => {
			"FDA Device Brand Name (FDA.G.k.12.r.4)"
		}
		("fda_device_information", "common_device_name") => {
			"FDA Common Device Name (FDA.G.k.12.r.5)"
		}
		("fda_device_information", "device_product_code") => {
			"FDA Device Product Code (FDA.G.k.12.r.6)"
		}
		("fda_device_information", "manufacturer_name") => {
			"FDA Device Manufacturer Name (FDA.G.k.12.r.7.1a)"
		}
		("fda_device_information", "manufacturer_address") => {
			"FDA Device Manufacturer Address (FDA.G.k.12.r.7.1b)"
		}
		("fda_device_information", "manufacturer_city") => {
			"FDA Device Manufacturer City (FDA.G.k.12.r.7.1c)"
		}
		("fda_device_information", "manufacturer_state") => {
			"FDA Device Manufacturer State (FDA.G.k.12.r.7.1d)"
		}
		("fda_device_information", "manufacturer_country") => {
			"FDA Device Manufacturer Country (FDA.G.k.12.r.7.1e)"
		}
		("fda_device_information", "device_usage") => {
			"FDA Device Usage (FDA.G.k.12.r.8)"
		}
		("fda_device_information", "device_lot_number") => {
			"FDA Device Lot Number (FDA.G.k.12.r.9)"
		}
		("fda_device_information", "operator_of_device") => {
			"FDA Operator of the Device (FDA.G.k.12.r.10)"
		}
		("fda_device_codes", "element") => "FDA Device Code Element",
		("fda_device_codes", "value_code") => "FDA Device Code Value",
		("drug_device_characteristics", "code") => "FDA Device Characteristic Code",
		("drug_device_characteristics", "code_system") => {
			"FDA Device Characteristic Code System"
		}
		("drug_device_characteristics", "code_display_name") => {
			"FDA Device Characteristic Code Display Name"
		}
		("drug_device_characteristics", "value_type") => {
			"FDA Device Characteristic Value Type"
		}
		("drug_device_characteristics", "value_value") => {
			"FDA Device Characteristic Text Value"
		}
		("drug_device_characteristics", "value_code") => {
			"FDA Device Characteristic Coded Value"
		}
		("drug_device_characteristics", "value_code_system") => {
			"FDA Device Characteristic Value Code System"
		}
		("drug_device_characteristics", "value_display_name") => {
			"FDA Device Characteristic Value Display Name"
		}
		("reactions", "primary_source_reaction") => {
			"Reaction/Event as Reported (E.i.1.1a)"
		}
		("reactions", "reaction_language") => "Reaction/Event Language (E.i.1.1b)",
		("reactions", "primary_source_reaction_translation") => {
			"Reaction/Event Translation (E.i.1.2)"
		}
		("reactions", "reaction_meddra_version") => {
			"Reaction MedDRA Version (E.i.2.1a)"
		}
		("reactions", "reaction_meddra_code") => "Reaction MedDRA Code (E.i.2.1b)",
		("reactions", "term_highlighted") => {
			"Term Highlighted by Reporter (E.i.3.1)"
		}
		("reactions", "serious") => "Serious Reaction/Event",
		("reactions", "criteria_death") => "Results in Death (E.i.3.2a)",
		("reactions", "criteria_life_threatening") => "Life Threatening (E.i.3.2b)",
		("reactions", "criteria_hospitalization") => {
			"Caused/Prolonged Hospitalisation (E.i.3.2c)"
		}
		("reactions", "criteria_disabling") => "Disabling/Incapacitating (E.i.3.2d)",
		("reactions", "criteria_congenital_anomaly") => {
			"Congenital Anomaly/Birth Defect (E.i.3.2e)"
		}
		("reactions", "criteria_other_medically_important") => {
			"Other Medically Important Condition (E.i.3.2f)"
		}
		("reactions", "required_intervention") => {
			"Required Intervention to Prevent Permanent Impairment (FDA.E.i.3.2h)"
		}
		("reactions", "expectedness") => "Expectedness",
		("reactions", "severity") => "Severity",
		("reactions", "mfds_device_ae_classification") => {
			"MFDS Device Adverse Event Classification (KR_DVC_AECL)"
		}
		("reactions", "mfds_device_ae_outcome") => {
			"MFDS Device Adverse Event Outcome (KR_DVC_AEOUT)"
		}
		("reactions", "mfds_device_cause_medical_device") => {
			"MFDS Device Cause: Medical Device (KR_DVC_CC_MD)"
		}
		("reactions", "mfds_device_cause_procedure_issue") => {
			"MFDS Device Cause: Procedure Issue (KR_DVC_CC_PI)"
		}
		("reactions", "mfds_device_cause_patient_condition") => {
			"MFDS Device Cause: Patient Condition (KR_DVC_CC_PC)"
		}
		("reactions", "mfds_device_cause_unable_to_assess") => {
			"MFDS Device Cause: Unable to Assess (KR_DVC_CC_UA)"
		}
		("reactions", "mfds_device_cause_other") => {
			"MFDS Device Cause: Other (KR_DVC_CC_OTH)"
		}
		("reactions", "mfds_device_action_reason") => {
			"MFDS Device Action Reason (KR_DVC_ACT_RSN)"
		}
		("reactions", "mfds_device_action_recall") => {
			"MFDS Device Action: Recall (KR_DVC_ACT_RC)"
		}
		("reactions", "mfds_device_action_repair") => {
			"MFDS Device Action: Repair (KR_DVC_ACT_RP)"
		}
		("reactions", "mfds_device_action_inspection") => {
			"MFDS Device Action: Inspection (KR_DVC_ACT_INSP)"
		}
		("reactions", "mfds_device_action_replacement") => {
			"MFDS Device Action: Replacement (KR_DVC_ACT_REPL)"
		}
		("reactions", "mfds_device_action_improvement") => {
			"MFDS Device Action: Improvement (KR_DVC_ACT_IMP)"
		}
		("reactions", "mfds_device_action_monitoring") => {
			"MFDS Device Action: Monitoring (KR_DVC_ACT_MON)"
		}
		("reactions", "mfds_device_action_notification") => {
			"MFDS Device Action: Notification (KR_DVC_ACT_NTF)"
		}
		("reactions", "mfds_device_action_label_change") => {
			"MFDS Device Action: Label Change (KR_DVC_ACT_CAS)"
		}
		("reactions", "mfds_device_action_other") => {
			"MFDS Device Action: Other (KR_DVC_ACT_OTH)"
		}
		("reactions", "start_date") => "Reaction Start Date (E.i.4)",
		("reactions", "end_date") => "Reaction End Date (E.i.5)",
		("reactions", "duration_value") => "Reaction Duration (E.i.6a)",
		("reactions", "duration_unit") => "Reaction Duration Unit (E.i.6b)",
		("reactions", "outcome") => "Reaction Outcome (E.i.7)",
		("reactions", "medical_confirmation") => "Medical Confirmation (E.i.8)",
		("reactions", "country_code") => "Reaction Country (E.i.9)",
		("test_results", "test_date") => "Test Date (F.r.1)",
		("test_results", "test_name") => "Test Name (F.r.2.1)",
		("test_results", "test_meddra_version") => "Test MedDRA Version (F.r.2.2a)",
		("test_results", "test_meddra_code") => "Test MedDRA Code (F.r.2.2b)",
		("test_results", "test_result_code") => "Test Result Code (F.r.3.1)",
		("test_results", "test_result_value") => "Test Result Value (F.r.3.2)",
		("test_results", "test_result_qualifier") => "Test Result Qualifier",
		("test_results", "test_result_unit") => "Test Result Unit (F.r.3.3)",
		("test_results", "result_unstructured") => {
			"Unstructured Test Result (F.r.3.4)"
		}
		("test_results", "normal_low_value") => "Normal Low Value (F.r.4)",
		("test_results", "normal_high_value") => "Normal High Value (F.r.5)",
		("test_results", "comments") => "Test Comments (F.r.6)",
		("test_results", "more_info_available") => {
			"More Information Available (F.r.7)"
		}
		("narrative_information", "case_narrative") => "Case Narrative (H.1)",
		("narrative_information", "reporter_comments") => {
			"Reporter's Comments (H.2)"
		}
		("sender_diagnoses", "diagnosis_meddra_version") => {
			"Sender Diagnosis MedDRA Version (H.3.r.1a)"
		}
		("sender_diagnoses", "diagnosis_meddra_code") => {
			"Sender Diagnosis (H.3.r.1b)"
		}
		("narrative_information", "sender_comments") => "Sender's Comments (H.4)",
		("narrative_information", "additional_information") => {
			"Additional Information (NR_SPONSOR)"
		}
		("case_summary_information", "language_code") => {
			"Case Summary Language (H.5.r.1b)"
		}
		("case_summary_information", "summary_text") => "Case Summary (H.5.r.1a)",
		("case_workflow_events", "from_status") => "Previous Workflow Status",
		("case_workflow_events", "from_role") => "Previous Assigned Role",
		("case_workflow_events", "from_user_id") => "Previous Assigned User",
		("case_workflow_events", "to_status") => "New Workflow Status",
		("case_workflow_events", "target_role") => "New Assigned Role",
		("case_workflow_events", "target_user_id") => "New Assigned User",
		("case_workflow_events", "comment") => "Workflow Comment",
		("case_workflow_events", "date_of_most_recent") => {
			"Workflow Date of Most Recent Information"
		}
		("case_workflow_events", "due_at") => "Workflow Due Date",
		("case_workflow_events", "acted_by") => "Workflow Actor",
		("case_workflow_events", "actor_role_id") => "Workflow Actor Role",
		("case_workflow_events", "used_admin_override") => "Admin Override Used",
		("case_workflow_events", "override_reason") => "Admin Override Reason",
		("case_submissions", "gateway") => "Submission Gateway",
		("case_submissions", "remote_submission_id") => "Remote Submission ID",
		("case_submissions", "status") => "Submission Status",
		("case_submissions", "xml_bytes") => "Submitted XML Size (Bytes)",
		("case_submissions", "submitted_by") => "Submitted By",
		("case_submissions", "submitted_at") => "Submitted At",
		("submission_events", "submission_id") => "Submission",
		("submission_events", "event_type") => "Submission Event Type",
		("submission_events", "event_data") => "Submission Event Details",
		("submission_acks", "submission_id") => "Submission",
		("submission_acks", "ack_level") => "Acknowledgement Level",
		("submission_acks", "success") => "Acknowledgement Successful",
		("submission_acks", "ack_code") => "Acknowledgement Code",
		("submission_acks", "ack_message") => "Acknowledgement Message",
		("submission_acks", "received_at") => "Acknowledgement Received At",
		("submission_acks", "raw_payload") => "Acknowledgement Payload",
		("case_versions", "version") => "Case Version",
		("case_versions", "snapshot") => "Case Version Snapshot",
		("case_versions", "changed_by") => "Case Version Changed By",
		("case_versions", "change_reason") => "Case Version Change Reason",
		("case_e2b_field_notations", "e2b_code") => "E2B Field Code",
		("case_e2b_field_notations", "notation") => "E2B Field Notation",
		("case_e2b_field_notations", "record_id") => "Notation Subject Record ID",
		("case_field_notations", "field_path") => "Case Field Path",
		("case_field_notations", "notation") => "Case Field Notation",
		("case_field_notations", "record_id") => "Notation Subject Record ID",
		("e_signatures", "signer_user_id") => "Electronic Signature User",
		("e_signatures", "signer_username") => "Electronic Signature Username",
		("e_signatures", "action") => "Electronic Signature Action",
		("e_signatures", "meaning") => "Electronic Signature Meaning",
		("e_signatures", "reason") => "Electronic Signature Reason",
		("e_signatures", "signature_method") => "Electronic Signature Method",
		("e_signatures", "signed_at") => "Electronically Signed At",
		("submission_dispatch_state", "submission_id") => "Submission",
		("submission_dispatch_state", "attempt_count") => "Dispatch Attempt Count",
		("submission_dispatch_state", "last_attempt_at") => {
			"Last Dispatch Attempt At"
		}
		("submission_dispatch_state", "last_error") => "Last Dispatch Error",
		("submission_dispatch_state", "next_retry_at") => "Next Dispatch Retry At",
		("submission_dispatch_state", "terminal_at") => "Dispatch Terminal At",
		("submission_idempotency", "submission_id") => "Submission",
		("submission_idempotency", "authority") => "Submission Authority",
		("submission_idempotency", "idempotency_key") => {
			"Submission Idempotency Key"
		}
		_ => return None,
	})
}

fn audit_field_label(
	table_name: &str,
	field: &str,
) -> Result<(&'static str, String)> {
	let page = audit_table_page(table_name).ok_or_else(|| {
		Error::Model(lib_core::model::Error::Store(format!(
			"unsupported case audit table mapping: {table_name}"
		)))
	})?;
	if let Some(label) = audit_business_field_label(table_name, field) {
		return Ok((page, label.to_string()));
	}
	if let Some(base_field) = field.strip_suffix("_null_flavor") {
		if let Some(label) = audit_business_field_label(table_name, base_field) {
			return Ok((page, format!("{label} — Null Flavor")));
		}
	}
	if let Some(base_field) = field.strip_suffix("_notation") {
		if let Some(label) = audit_business_field_label(table_name, base_field) {
			return Ok((page, format!("{label} — Notation")));
		}
	}
	let metadata_label = match field {
		"id" => Some("Record metadata: ID"),
		"case_id" => Some("Record metadata: Case ID"),
		"organization_id" => Some("Record metadata: Organization ID"),
		"patient_id" => Some("Record metadata: Patient ID"),
		"parent_id" => Some("Record metadata: Parent ID"),
		"death_info_id" => Some("Record metadata: Death Information ID"),
		"drug_id" => Some("Record metadata: Drug ID"),
		"device_id" => Some("Record metadata: Device ID"),
		"reaction_id" => Some("Record metadata: Reaction ID"),
		"narrative_id" => Some("Record metadata: Narrative ID"),
		"study_information_id" => Some("Record metadata: Study Information ID"),
		"drug_reaction_assessment_id" => {
			Some("Record metadata: Drug Reaction Assessment ID")
		}
		"source_sender_presave_id" => {
			Some("Record metadata: Source Sender Presave ID")
		}
		"source_reporter_presave_id" => {
			Some("Record metadata: Source Reporter Presave ID")
		}
		"source_product_presave_id" => {
			Some("Record metadata: Source Product Presave ID")
		}
		"source_study_presave_id" => {
			Some("Record metadata: Source Study Presave ID")
		}
		"source_narrative_presave_id" => {
			Some("Record metadata: Source Narrative Presave ID")
		}
		"created_at" => Some("Record metadata: Created At"),
		"updated_at" => Some("Record metadata: Updated At"),
		"created_by" => Some("Record metadata: Created By"),
		"updated_by" => Some("Record metadata: Updated By"),
		"sequence_number" => Some("Record metadata: Sequence Number"),
		"deleted" => Some("Record metadata: Deleted"),
		"raw_xml" => Some("Record metadata: Raw XML"),
		"dirty_c" => Some("Record metadata: Section C Dirty"),
		"dirty_d" => Some("Record metadata: Section D Dirty"),
		"dirty_e" => Some("Record metadata: Section E Dirty"),
		"dirty_f" => Some("Record metadata: Section F Dirty"),
		"dirty_g" => Some("Record metadata: Section G Dirty"),
		"dirty_h" => Some("Record metadata: Section H Dirty"),
		_ => None,
	};
	if let Some(label) = metadata_label {
		return Ok((page, label.to_string()));
	}
	let item = match (table_name, field) {
		("cases", "report_year") => "Report Year (REPORT_YEAR)".to_string(),
		("cases", "report_type") => "Type of Report (C.1.3)".to_string(),
		("cases", "date_of_most_recent_information") => {
			"Date of Most Recent Information for This Report (C.1.5)".to_string()
		}
		("cases", "safety_report_id") => {
			"Sender's Safety Report Unique Identifier (C.1.1)".to_string()
		}
		("safety_report_identification", "report_type") => {
			"Type of Report (C.1.3)".to_string()
		}
		("safety_report_identification", "date_of_most_recent_information") => {
			"Date of Most Recent Information for This Report (C.1.5)".to_string()
		}
		_ => {
			return Err(Error::Model(lib_core::model::Error::Store(format!(
				"unsupported case audit field mapping: {table_name}.{field}"
			))));
		}
	};
	Ok((page, item))
}

fn audit_snapshot(log: &AuditLog) -> Option<&serde_json::Map<String, JsonValue>> {
	(if log.action == "DELETE" {
		log.old_values.as_ref()
	} else {
		log.new_values.as_ref()
	})
	.and_then(JsonValue::as_object)
}

fn audit_snapshot_text<'a>(log: &'a AuditLog, field: &str) -> Option<&'a str> {
	audit_snapshot(log)?.get(field)?.as_str()
}

fn audit_parent_record_id(log: &AuditLog) -> Option<Uuid> {
	[
		"drug_reaction_assessment_id",
		"device_id",
		"death_info_id",
		"narrative_id",
		"parent_id",
		"patient_id",
		"study_information_id",
		"drug_id",
		"case_id",
	]
	.into_iter()
	.find_map(|field| audit_snapshot_text(log, field)?.parse().ok())
}

fn audit_record_snapshot_at<'logs>(
	logs_by_record: &std::collections::HashMap<Uuid, Vec<&'logs AuditLog>>,
	record_id: Uuid,
	event_id: i64,
) -> Option<&'logs AuditLog> {
	logs_by_record.get(&record_id).and_then(|snapshots| {
		let end = snapshots.partition_point(|candidate| candidate.id <= event_id);
		end.checked_sub(1).map(|index| snapshots[index])
	})
}

fn audit_row_numbers(
	log: &AuditLog,
	logs_by_record: &std::collections::HashMap<Uuid, Vec<&AuditLog>>,
) -> [String; 3] {
	let mut numbers = Vec::new();
	let mut visited = std::collections::HashSet::new();
	let mut current = Some(log);
	while let Some(snapshot_log) = current {
		if !visited.insert(snapshot_log.record_id) {
			break;
		}
		if let Some(sequence) = audit_snapshot(snapshot_log)
			.and_then(|snapshot| snapshot.get("sequence_number"))
			.and_then(JsonValue::as_i64)
		{
			numbers.push(sequence.to_string());
		}
		if snapshot_log.table_name == "drug_reaction_assessments" {
			let reaction_snapshot = audit_snapshot_text(snapshot_log, "reaction_id")
				.and_then(|reaction_id| reaction_id.parse().ok())
				.and_then(|reaction_id| {
					audit_record_snapshot_at(logs_by_record, reaction_id, log.id)
				});
			if let Some(sequence) = reaction_snapshot
				.and_then(audit_snapshot)
				.and_then(|snapshot| snapshot.get("sequence_number"))
				.and_then(JsonValue::as_i64)
			{
				numbers.push(sequence.to_string());
			}
		}
		current = audit_parent_record_id(snapshot_log).and_then(|parent_id| {
			audit_record_snapshot_at(logs_by_record, parent_id, log.id)
		});
	}
	numbers.reverse();
	let mut rows = std::array::from_fn(|_| String::new());
	for (target, value) in rows.iter_mut().zip(numbers.into_iter().take(3)) {
		*target = value;
	}
	rows
}

fn changed_field_entries(log: &AuditLog) -> Vec<(String, JsonValue)> {
	log.changed_fields
		.as_ref()
		.and_then(JsonValue::as_object)
		.map(|object| {
			object
				.iter()
				.map(|(key, value)| (key.clone(), value.clone()))
				.collect()
		})
		.unwrap_or_default()
}

fn json_display_value(value: &JsonValue) -> String {
	match value {
		JsonValue::String(value) => value.clone(),
		JsonValue::Null => String::new(),
		other => other.to_string(),
	}
}

fn audit_display_value(action: &str, diff: &JsonValue) -> String {
	let preferred = if action == "DELETE" { "old" } else { "new" };
	diff.get(preferred)
		.map(json_display_value)
		.unwrap_or_default()
}

fn audit_notation_value(action: &str, field: &str, diff: &JsonValue) -> String {
	if field.ends_with("_notation") {
		audit_display_value(action, diff)
	} else {
		String::new()
	}
}

fn audit_null_flavor_value(action: &str, field: &str, diff: &JsonValue) -> String {
	if field.ends_with("_null_flavor") {
		audit_display_value(action, diff)
	} else {
		String::new()
	}
}

fn audit_reason_value(action: &str, reason: Option<String>) -> String {
	reason.unwrap_or_else(|| {
		if action == "CREATE" {
			"Initial Data".to_string()
		} else {
			String::new()
		}
	})
}

/// GET /api/audit-logs
/// List all audit logs with optional filtering
/// **Requires AuditLog.List permission**
pub async fn list_audit_logs(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<AuditLog>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest list_audit_logs", "HANDLER");

	with_authorized_audit_log_collection(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let params =
				ParamsList::<AuditLogFilter>::from_raw_query(raw_query.as_deref())
					.map_err(|message| Error::BadRequest { message })?;
			let logs =
				AuditLogBmc::list(ctx, mm, params.filters, params.list_options)
					.await
					.map_err(Error::Model)?;
			Ok((StatusCode::OK, Json(DataRestResult { data: logs })))
		})
	})
	.await
}

/// GET /api/audit-logs/by-record/{table_name}/{record_id}
/// List audit logs for a specific record
/// **Requires AuditLog.List permission**
pub async fn list_audit_logs_by_record(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path((table_name, record_id)): Path<(String, Uuid)>,
	Query(query): Query<AuditRecordQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<AuditLog>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!(
		"{:<12} - rest list_audit_logs_by_record table={} id={}",
		"HANDLER",
		table_name,
		record_id
	);

	with_authorized_audit_log_collection(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let mut logs =
				AuditLogBmc::list_by_record(ctx, mm, &table_name, record_id)
					.await
					.map_err(Error::Model)?;
			if let Some(field) = query
				.field
				.as_deref()
				.map(str::trim)
				.filter(|field| !field.is_empty())
			{
				logs.retain(|log| {
					field
						.split(',')
						.any(|field| audit_log_touches_field(log, field.trim()))
				});
			}
			Ok((StatusCode::OK, Json(DataRestResult { data: logs })))
		})
	})
	.await
}

/// GET /api/cases/{case_id}/audit-trail
/// Reference-style field-level audit projection for a case.
/// **Requires AuditLog.List permission**
pub async fn list_case_audit_trail(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<CaseAuditTrailRow>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!(
		"{:<12} - rest list_case_audit_trail case_id={}",
		"HANDLER",
		case_id
	);

	with_authorized_case_audit_read(&ctx, &snapshot, &mm, case_id, move |ctx, mm| {
		Box::pin(async move {
			let params =
				ParamsList::<AuditLogFilter>::from_raw_query(raw_query.as_deref())
					.map_err(|message| Error::BadRequest { message })?;
			if let Some(order_bys) = params
				.list_options
				.as_ref()
				.and_then(|options| options.order_bys.as_ref())
			{
				for order_by in order_bys {
					match order_by {
						OrderBy::Desc(field) if field == "created_at" => {}
						_ => {
							return Err(Error::BadRequest {
								message: "case audit trail supports only descending created_at order"
									.to_string(),
							});
						}
					}
				}
			}
			let logs = AuditLogBmc::list_by_record(ctx, mm, "cases", case_id)
				.await
				.map_err(Error::Model)?;
			let mut logs_by_record = std::collections::HashMap::new();
			for log in &logs {
				logs_by_record
					.entry(log.record_id)
					.or_insert_with(Vec::new)
					.push(log);
			}
			for snapshots in logs_by_record.values_mut() {
				snapshots.sort_unstable_by_key(|log| log.id);
			}
			let mut rows = Vec::new();
			for log in &logs {
				let [row_no1, row_no2, row_no3] =
					audit_row_numbers(log, &logs_by_record);
				for (field, diff) in changed_field_entries(log) {
					let (page, item) = audit_field_label(&log.table_name, &field)?;
					rows.push(CaseAuditTrailRow {
						no: log.id,
						audit_log_id: log.id,
						date_time: log.created_at,
						user_display: log.user_display.clone(),
						page: page.to_string(),
						item,
						row_no1: row_no1.clone(),
						row_no2: row_no2.clone(),
						row_no3: row_no3.clone(),
						value: audit_display_value(&log.action, &diff),
						notation: audit_notation_value(&log.action, &field, &diff),
						null_flavor: audit_null_flavor_value(
							&log.action,
							&field,
							&diff,
						),
						reason: audit_reason_value(
							&log.action,
							log.reason_for_change.clone(),
						),
						e_signature_id: log.e_signature_id,
					});
				}
			}
			rows.sort_by(|left, right| {
				right
					.date_time
					.cmp(&left.date_time)
					.then_with(|| right.audit_log_id.cmp(&left.audit_log_id))
					.then_with(|| left.item.cmp(&right.item))
			});
			if let Some(options) = params.list_options {
				if let Some(offset) = options.offset {
					let offset =
						usize::try_from(offset).map_err(|_| Error::BadRequest {
							message: "audit offset must be non-negative".to_string(),
						})?;
					rows = rows.into_iter().skip(offset).collect();
				}
				if let Some(limit) = options.limit {
					let limit =
						usize::try_from(limit).map_err(|_| Error::BadRequest {
							message: "audit limit must be non-negative".to_string(),
						})?;
					rows.truncate(limit);
				}
			}
			Ok((StatusCode::OK, Json(DataRestResult { data: rows })))
		})
	})
	.await
}

/// GET /api/cases/{case_id}/versions
/// List all versions for a specific case
/// **Requires AuditLog.Read permission**
pub async fn list_case_versions(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<CaseVersion>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!(
		"{:<12} - rest list_case_versions case_id={}",
		"HANDLER",
		case_id
	);

	with_authorized_case_audit_read(&ctx, &snapshot, &mm, case_id, move |ctx, mm| {
		Box::pin(async move {
			let versions = CaseVersionBmc::list_by_case(ctx, mm, case_id)
				.await
				.map_err(Error::Model)?;
			Ok((StatusCode::OK, Json(DataRestResult { data: versions })))
		})
	})
	.await
}

/// GET /api/audit-logs/verify-integrity
/// Verifies the append-only audit hash chain integrity.
/// **Requires AuditLog.List permission**
pub async fn verify_audit_log_integrity(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(
	StatusCode,
	Json<DataRestResult<AuditChainVerificationReport>>,
)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest verify_audit_log_integrity", "HANDLER");

	with_authorized_audit_log_collection(&ctx, &snapshot, &mm, |ctx, mm| {
		Box::pin(async move {
			let report = AuditLogBmc::verify_hash_chain(ctx, mm)
				.await
				.map_err(Error::Model)?;
			Ok((StatusCode::OK, Json(DataRestResult { data: report })))
		})
	})
	.await
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	fn log(
		id: i64,
		table_name: &str,
		record_id: Uuid,
		action: &str,
		old_values: Option<JsonValue>,
		new_values: Option<JsonValue>,
	) -> AuditLog {
		AuditLog {
			id,
			organization_id: Uuid::nil(),
			table_name: table_name.to_string(),
			record_id,
			action: action.to_string(),
			user_id: Uuid::nil(),
			reason_for_change: None,
			change_category: None,
			e_signature_id: None,
			user_display: None,
			changed_fields: None,
			old_values,
			new_values,
			ip_address: None,
			user_agent: None,
			prev_hash: None,
			entry_hash: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
		}
	}

	#[test]
	fn audit_field_label_maps_reference_ci_fields() {
		assert_eq!(
			audit_field_label("cases", "report_type").unwrap(),
			("CI (C.1)", "Type of Report (C.1.3)".to_string())
		);
		assert_eq!(
			audit_field_label("cases", "mfds_report_type").unwrap(),
			(
				"CI (C.1)",
				"MFDS Report Type (MFDS_REPORT_TYPE)".to_string()
			)
		);
		assert_eq!(
			audit_field_label("cases", "fda_report_type").unwrap(),
			("CI (C.1)", "FDA Report Type (FDA_REPORT_TYPE)".to_string())
		);
		assert_eq!(
			audit_field_label("study_information", "study_name").unwrap(),
			("SI (C.5)", "Study Name (C.5.2)".to_string())
		);
		assert_eq!(
			audit_field_label("drug_information", "action_taken").unwrap(),
			("DG (G.k)", "Action Taken with Drug (G.k.8)".to_string())
		);
		assert_eq!(
			audit_field_label("drug_active_substances", "substance_termid").unwrap(),
			("DG (G.k)", "Substance Term ID (G.k.2.3.r.2b)".to_string())
		);
		assert_eq!(
			audit_field_label("receiver_information", "receiver_type").unwrap(),
			("SD (A.1 Receiver)", "Receiver Type".to_string())
		);
		assert_eq!(
			audit_field_label("message_headers", "message_sender_identifier")
				.unwrap(),
			("SD (N)", "Message Sender Identifier (N.2.r.2)".to_string())
		);
		assert_eq!(
			audit_field_label("reactions", "mfds_device_action_recall").unwrap(),
			(
				"AE (E.i)",
				"MFDS Device Action: Recall (KR_DVC_ACT_RC)".to_string()
			)
		);
		assert_eq!(
			audit_field_label("fda_device_information", "device_brand_name")
				.unwrap(),
			(
				"DG (G.k)",
				"FDA Device Brand Name (FDA.G.k.12.r.4)".to_string()
			)
		);
		assert_eq!(
			audit_field_label("case_workflow_events", "due_at").unwrap(),
			("WF", "Workflow Due Date".to_string())
		);
		assert_eq!(
			audit_field_label("submission_acks", "ack_code").unwrap(),
			("RE", "Acknowledgement Code".to_string())
		);
		assert_eq!(
			audit_field_label("dosage_information", "route_termid_code_system")
				.unwrap(),
			("DG (G.k)", "Route Term ID Code System".to_string())
		);
		assert_eq!(
			audit_field_label("primary_sources", "qualification_kr1").unwrap(),
			(
				"RP (C.2.r)",
				"MFDS Reporter Qualification (C.2.r.4.KR.1)".to_string()
			)
		);
	}

	#[tokio::test]
	#[ignore = "requires an explicit dedicated UI database; read-only schema/history coverage"]
	async fn audit_field_labels_cover_live_schema_and_history() {
		let url = std::env::var("AUDIT_MAPPING_DATABASE_URL").expect(
			"set AUDIT_MAPPING_DATABASE_URL to a dedicated e2br3_ui_ database",
		);
		let pool = sqlx::PgPool::connect(&url).await.unwrap();
		let database: String = sqlx::query_scalar("SELECT current_database()")
			.fetch_one(&pool)
			.await
			.unwrap();
		assert!(
			database.starts_with("e2br3_ui_"),
			"dedicated UI database required"
		);
		let fields: Vec<(String, String)> = sqlx::query_as(
			r#"
			WITH RECURSIVE records(record_id) AS (
				SELECT record_id FROM audit_logs WHERE table_name = 'cases'
				UNION
				SELECT child.record_id FROM audit_logs child JOIN records parent
				 ON child.parent_record_ids @> ARRAY[parent.record_id]
			)
			SELECT c.relname::text, a.attname::text
			FROM pg_attribute a JOIN pg_class c ON c.oid = a.attrelid
			JOIN pg_namespace n ON n.oid = c.relnamespace
			WHERE n.nspname = 'public' AND a.attnum > 0 AND NOT a.attisdropped
			AND EXISTS (SELECT 1 FROM pg_trigger t JOIN pg_proc p ON p.oid = t.tgfoid
			 WHERE t.tgrelid = c.oid AND NOT t.tgisinternal
			 AND p.proname LIKE 'audit_trigger_function%')
			AND (c.relname = 'cases' OR EXISTS (
			 SELECT 1 FROM pg_attribute x WHERE x.attrelid = c.oid
			 AND x.attnum > 0 AND NOT x.attisdropped
			 AND cardinality(public.audit_parent_record_ids('{}'::jsonb,
			 jsonb_build_object(x.attname, '00000000-0000-0000-0000-000000000001'))) > 0))
			UNION
			SELECT l.table_name, f.key FROM audit_logs l
			CROSS JOIN LATERAL jsonb_object_keys(l.changed_fields) AS f(key)
			WHERE l.record_id IN (SELECT record_id FROM records)
			ORDER BY 1, 2
		"#,
		)
		.fetch_all(&pool)
		.await
		.unwrap();
		assert!(!fields.is_empty(), "no audit fields were inspected");
		let missing: Vec<String> = fields
			.iter()
			.filter(|(table, field)| audit_field_label(table, field).is_err())
			.map(|(table, field)| format!("{table}.{field}"))
			.collect();
		assert!(
			missing.is_empty(),
			"unmapped audit fields: {}",
			missing.join(", ")
		);
		println!("Verified {} schema/history audit fields", fields.len());
	}

	#[test]
	fn audit_field_label_rejects_unknown_table_and_field() {
		assert!(matches!(
			audit_field_label("cases", "future_unmapped_field"),
			Err(Error::Model(lib_core::model::Error::Store(message)))
				if message.contains("cases.future_unmapped_field")
		));
		assert!(matches!(
			audit_field_label("future_case_child", "id"),
			Err(Error::Model(lib_core::model::Error::Store(message)))
				if message.contains("future_case_child")
		));
	}

	#[test]
	fn audit_display_value_prefers_new_value_except_delete() {
		let diff = json!({ "old": "1", "new": "2" });
		assert_eq!(audit_display_value("UPDATE", &diff), "2");
		assert_eq!(audit_display_value("CREATE", &diff), "2");
		assert_eq!(audit_display_value("DELETE", &diff), "1");
	}

	#[test]
	fn audit_display_value_does_not_fallback_to_the_other_snapshot() {
		assert_eq!(audit_display_value("UPDATE", &json!({ "old": "1" })), "");
		assert_eq!(audit_display_value("DELETE", &json!({ "new": "2" })), "");
	}

	#[test]
	fn audit_reason_defaults_create_to_initial_data() {
		assert_eq!(audit_reason_value("CREATE", None), "Initial Data");
		assert_eq!(
			audit_reason_value("CREATE", Some("Imported row".to_string())),
			"Imported row"
		);
		assert_eq!(audit_reason_value("UPDATE", None), "");
	}

	#[test]
	fn audit_row_numbers_use_the_parent_sequence_at_the_event_time() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let dose_id = Uuid::new_v4();
		let device_id = Uuid::new_v4();
		let device_code_id = Uuid::new_v4();
		let cycle_parent_id = Uuid::new_v4();
		let cycle_child_id = Uuid::new_v4();
		let logs = vec![
			log(
				1,
				"drug_information",
				drug_id,
				"CREATE",
				None,
				Some(json!({
					"id": drug_id, "case_id": case_id, "sequence_number": 1
				})),
			),
			log(
				2,
				"dosage_information",
				dose_id,
				"CREATE",
				None,
				Some(json!({
					"id": dose_id, "drug_id": drug_id, "sequence_number": 2
				})),
			),
			log(
				3,
				"drug_information",
				drug_id,
				"UPDATE",
				Some(json!({
					"id": drug_id, "case_id": case_id, "sequence_number": 1
				})),
				Some(json!({
					"id": drug_id, "case_id": case_id, "sequence_number": 5
				})),
			),
			log(
				4,
				"dosage_information",
				dose_id,
				"DELETE",
				Some(json!({
					"id": dose_id, "drug_id": drug_id, "sequence_number": 2
				})),
				None,
			),
			log(
				5,
				"fda_device_information",
				device_id,
				"CREATE",
				None,
				Some(json!({
					"id": device_id, "drug_id": drug_id, "sequence_number": 3
				})),
			),
			log(
				6,
				"fda_device_codes",
				device_code_id,
				"DELETE",
				Some(json!({
					"id": device_code_id, "device_id": device_id, "sequence_number": 4
				})),
				None,
			),
			log(
				7,
				"drug_information",
				cycle_parent_id,
				"CREATE",
				None,
				Some(json!({
					"id": cycle_parent_id,
					"parent_id": cycle_child_id,
					"sequence_number": 9
				})),
			),
			log(
				8,
				"drug_information",
				cycle_child_id,
				"CREATE",
				None,
				Some(json!({
					"id": cycle_child_id,
					"parent_id": cycle_parent_id,
					"sequence_number": 8
				})),
			),
		];
		let mut logs_by_record = std::collections::HashMap::new();
		for log in &logs {
			logs_by_record
				.entry(log.record_id)
				.or_insert_with(Vec::new)
				.push(log);
		}
		for snapshots in logs_by_record.values_mut() {
			snapshots.sort_unstable_by_key(|log| log.id);
		}

		assert_eq!(audit_row_numbers(&logs[1], &logs_by_record), ["1", "2", ""]);
		assert_eq!(audit_row_numbers(&logs[3], &logs_by_record), ["5", "2", ""]);
		assert_eq!(
			audit_row_numbers(&logs[5], &logs_by_record),
			["5", "3", "4"]
		);
		assert_eq!(audit_row_numbers(&logs[7], &logs_by_record), ["9", "8", ""]);
	}

	#[test]
	fn audit_row_numbers_include_reaction_sequence_at_the_event_time() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let reaction_id = Uuid::new_v4();
		let assessment_id = Uuid::new_v4();
		let relatedness_id = Uuid::new_v4();
		let logs = vec![
			log(
				20,
				"reactions",
				reaction_id,
				"CREATE",
				None,
				Some(json!({
					"id": reaction_id, "case_id": case_id, "sequence_number": 2
				})),
			),
			log(
				21,
				"drug_information",
				drug_id,
				"CREATE",
				None,
				Some(json!({
					"id": drug_id, "case_id": case_id, "sequence_number": 1
				})),
			),
			log(
				22,
				"drug_reaction_assessments",
				assessment_id,
				"CREATE",
				None,
				Some(json!({
					"id": assessment_id, "drug_id": drug_id, "reaction_id": reaction_id
				})),
			),
			log(
				23,
				"relatedness_assessments",
				relatedness_id,
				"CREATE",
				None,
				Some(json!({
					"id": relatedness_id,
					"drug_reaction_assessment_id": assessment_id,
					"sequence_number": 3
				})),
			),
			log(
				24,
				"reactions",
				reaction_id,
				"UPDATE",
				Some(json!({
					"id": reaction_id, "case_id": case_id, "sequence_number": 2
				})),
				Some(json!({
					"id": reaction_id, "case_id": case_id, "sequence_number": 4
				})),
			),
			log(
				25,
				"drug_reaction_assessments",
				assessment_id,
				"UPDATE",
				Some(json!({
					"id": assessment_id, "drug_id": drug_id, "reaction_id": reaction_id,
					"expectedness": "1"
				})),
				Some(json!({
					"id": assessment_id, "drug_id": drug_id, "reaction_id": reaction_id,
					"expectedness": "2"
				})),
			),
			log(
				26,
				"relatedness_assessments",
				relatedness_id,
				"UPDATE",
				Some(json!({
					"id": relatedness_id,
					"drug_reaction_assessment_id": assessment_id,
					"sequence_number": 3,
					"source_of_assessment": "1"
				})),
				Some(json!({
					"id": relatedness_id,
					"drug_reaction_assessment_id": assessment_id,
					"sequence_number": 3,
					"source_of_assessment": "2"
				})),
			),
		];
		let mut logs_by_record = std::collections::HashMap::new();
		for log in &logs {
			logs_by_record
				.entry(log.record_id)
				.or_insert_with(Vec::new)
				.push(log);
		}
		for snapshots in logs_by_record.values_mut() {
			snapshots.sort_unstable_by_key(|log| log.id);
		}

		assert_eq!(audit_row_numbers(&logs[2], &logs_by_record), ["1", "2", ""]);
		assert_eq!(
			audit_row_numbers(&logs[3], &logs_by_record),
			["1", "2", "3"]
		);
		assert_eq!(audit_row_numbers(&logs[5], &logs_by_record), ["1", "4", ""]);
		assert_eq!(
			audit_row_numbers(&logs[6], &logs_by_record),
			["1", "4", "3"]
		);
	}
}
