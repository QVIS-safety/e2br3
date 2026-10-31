//! Characterization tests for the one-to-one presence/value rules inside
//! `collect_ich_issues` (C.1.2 / C.1.3 / C.1.4 / C.1.5 / C.1.7).
//!
//! These freeze *current* behavior (code + path) so the
//! table-driven refactor can be proven to change nothing. Deliberately
//! excluded from scope: C.1.1 (fires outside the `if let Some(report)`
//! block), cross-field date rules (`*.FUTURE_DATE`, `*.AFTER_*`), and the
//! C.1.7 nullFlavor parity with the dictionary.
use super::*;
use lib_core::model::case::Case;
use lib_core::model::case_identifiers::OtherCaseIdentifier;
use lib_core::model::message_header::MessageHeader;
use lib_core::model::patient::PatientInformation;
use lib_core::model::safety_report::{
	DocumentsHeldBySender, LiteratureReference, PrimarySource,
	SafetyReportIdentification, SenderInformation, StudyFdaCrossReportedInd,
	StudyInformation, StudyRegistrationNumber,
};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;

const TARGET_CODES: &[&str] = &[
	"ICH.C.1.2.REQUIRED",
	"ICH.C.1.3.REQUIRED",
	"ICH.C.1.4.REQUIRED",
	"ICH.C.1.5.REQUIRED",
	"ICH.C.1.7.REQUIRED",
];

const INDEXED_CODES: &[&str] = &[
	"ICH.C.1.6.1.r.1.REQUIRED",
	"ICH.C.1.9.1.r.1.REQUIRED",
	"ICH.C.1.9.1.r.2.REQUIRED",
];

const STUDY_CODES: &[&str] = &["ICH.C.5.3.REQUIRED", "ICH.C.5.4.REQUIRED"];

const LENGTH_CODES: &[&str] = &[
	"ICH.C.1.1.LENGTH.MAX",
	"ICH.C.1.3.LENGTH.MAX",
	"ICH.C.1.6.1.r.1.LENGTH.MAX",
	"ICH.C.1.8.1.LENGTH.MAX",
	"ICH.C.1.8.2.LENGTH.MAX",
	"ICH.C.1.9.1.r.1.LENGTH.MAX",
	"ICH.C.1.9.1.r.2.LENGTH.MAX",
	"ICH.C.1.10.r.LENGTH.MAX",
	"ICH.C.1.11.1.LENGTH.MAX",
	"ICH.C.1.11.2.LENGTH.MAX",
	"ICH.C.5.3.LENGTH.MAX",
	"ICH.C.5.4.LENGTH.MAX",
];

const C23_LENGTH_CODES: &[&str] = &[
	"ICH.C.2.r.1.1.LENGTH.MAX",
	"ICH.C.2.r.1.2.LENGTH.MAX",
	"ICH.C.2.r.1.3.LENGTH.MAX",
	"ICH.C.2.r.1.4.LENGTH.MAX",
	"ICH.C.2.r.2.1.LENGTH.MAX",
	"ICH.C.2.r.2.2.LENGTH.MAX",
	"ICH.C.2.r.2.3.LENGTH.MAX",
	"ICH.C.2.r.2.4.LENGTH.MAX",
	"ICH.C.2.r.2.5.LENGTH.MAX",
	"ICH.C.2.r.2.6.LENGTH.MAX",
	"ICH.C.2.r.2.7.LENGTH.MAX",
	"ICH.C.2.r.3.LENGTH.MAX",
	"ICH.C.2.r.4.LENGTH.MAX",
	"ICH.C.2.r.5.LENGTH.MAX",
	"ICH.C.3.1.LENGTH.MAX",
	"ICH.C.3.2.LENGTH.MAX",
	"ICH.C.3.3.1.LENGTH.MAX",
	"ICH.C.3.3.2.LENGTH.MAX",
	"ICH.C.3.3.3.LENGTH.MAX",
	"ICH.C.3.3.4.LENGTH.MAX",
	"ICH.C.3.3.5.LENGTH.MAX",
	"ICH.C.3.4.1.LENGTH.MAX",
	"ICH.C.3.4.2.LENGTH.MAX",
	"ICH.C.3.4.3.LENGTH.MAX",
	"ICH.C.3.4.4.LENGTH.MAX",
	"ICH.C.3.4.5.LENGTH.MAX",
	"ICH.C.3.4.6.LENGTH.MAX",
	"ICH.C.3.4.7.LENGTH.MAX",
	"ICH.C.3.4.8.LENGTH.MAX",
	"ICH.C.5.2.LENGTH.MAX",
];

const C45_LENGTH_CODES: &[&str] = &[
	"ICH.C.4.r.1.LENGTH.MAX",
	"ICH.C.5.1.r.1.LENGTH.MAX",
	"ICH.C.5.1.r.2.LENGTH.MAX",
];

fn dummy_case() -> Case {
	Case {
		id: Uuid::nil(),
		organization_id: Uuid::nil(),
		dg_prd_key: None,
		status: String::new(),
		status_before_lock: None,
		review_receivers_json: None,
		workflow_routes_json: None,
		workflow_status: String::new(),
		workflow_assigned_role: None,
		workflow_assigned_user_id: None,
		workflow_due_at: None,
		workflow_description: None,
		workflow_updated_at: OffsetDateTime::UNIX_EPOCH,
		mfds_report_type: None,
		fda_report_type: None,
		report_year: None,
		created_by: Uuid::nil(),
		updated_by: None,
		submitted_by: None,
		submitted_at: None,
		raw_xml: None,
		dirty_c: false,
		dirty_d: false,
		dirty_e: false,
		dirty_f: false,
		dirty_g: false,
		dirty_h: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
	}
}

pub(super) fn base_report() -> SafetyReportIdentification {
	SafetyReportIdentification {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		safety_report_id: None,
		version: 0,
		transmission_date: None,
		report_type: None,
		date_first_received_from_source: None,
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: None,
		fulfil_expedited_criteria_null_flavor: None,
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
		combination_product_report_indicator_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn ctx_with(report: SafetyReportIdentification) -> ValidationContext {
	ValidationContext {
		vocabulary: Default::default(),
		case: dummy_case(),
		safety_report: Some(report),
		message_header: None,
		sender: None,
		patient: None,
		narrative: None,
		sender_diagnoses: Vec::new(),
		case_summaries: Vec::new(),
		medical_history: Vec::new(),
		past_drugs: Vec::new(),
		death_info: None,
		reported_causes_of_death: Vec::new(),
		autopsy_causes_of_death: Vec::new(),
		parents: Vec::new(),
		parent_medical_history: Vec::new(),
		parent_past_drugs: Vec::new(),
		primary_sources: Vec::new(),
		documents_held_by_sender: Vec::new(),
		literature_references: Vec::new(),
		other_case_identifiers: Vec::new(),
		linked_report_numbers: Vec::new(),
		studies: Vec::new(),
		study_registrations: Vec::new(),
		reactions: Vec::new(),
		tests: Vec::new(),
		drugs: Vec::new(),
		active_substances: Vec::new(),
		indications: Vec::new(),
		dosages: Vec::new(),
		drug_reaction_assessments: Vec::new(),
		relatedness_assessments: Vec::new(),
		patient_identifiers: Vec::new(),
	}
}

/// Runs `collect_ich_issues` and returns only the in-scope C.1 value rules
/// as a sorted `(code, path)` snapshot. Issue *ordering* is not a
/// contract (`build_report` aggregates by section), so we compare as a set.
fn snapshot(report: SafetyReportIdentification) -> Vec<(String, String)> {
	let mut issues = Vec::new();
	collect_ich_issues(&ctx_with(report), &mut issues);
	let mut out: Vec<(String, String)> = issues
		.into_iter()
		.filter(|issue| TARGET_CODES.contains(&issue.code.as_str()))
		.map(|issue| (issue.code, issue.path))
		.collect();
	out.sort();
	out
}

fn issue(code: &str, path: &str) -> (String, String) {
	(code.to_string(), path.to_string())
}

/// Sorted `(code, path)` snapshot filtered to `targets`, for
/// contexts built with repeated-field fixtures.
fn filtered(ctx: &ValidationContext, targets: &[&str]) -> Vec<(String, String)> {
	let mut issues = Vec::new();
	collect_ich_issues(ctx, &mut issues);
	let mut out: Vec<(String, String)> = issues
		.into_iter()
		.filter(|issue| targets.contains(&issue.code.as_str()))
		.map(|issue| (issue.code, issue.path))
		.collect();
	out.sort();
	out
}

pub(super) fn document(title: Option<&str>) -> DocumentsHeldBySender {
	DocumentsHeldBySender {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		title: title.map(str::to_string),
		document_base64: None,
		file_name: None,
		media_type: None,
		representation: None,
		compression: None,
		sequence_number: 0,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn other_identifier(
	source: &str,
	case_identifier: &str,
) -> OtherCaseIdentifier {
	OtherCaseIdentifier {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		sequence_number: 0,
		source_of_identifier: source.to_string(),
		case_identifier: case_identifier.to_string(),
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn linked_report_number(value: String) -> LinkedReportNumber {
	LinkedReportNumber {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		sequence_number: 1,
		linked_report_number: value,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(in crate::case::sections) fn primary_source() -> PrimarySource {
	PrimarySource {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		source_reporter_presave_id: None,
		sequence_number: 0,
		reporter_title: None,
		reporter_title_null_flavor: None,
		reporter_given_name: None,
		reporter_given_name_null_flavor: None,
		reporter_middle_name: None,
		reporter_middle_name_null_flavor: None,
		reporter_family_name: None,
		reporter_family_name_null_flavor: None,
		organization: None,
		organization_null_flavor: None,
		department: None,
		department_null_flavor: None,
		street: None,
		street_null_flavor: None,
		city: None,
		city_null_flavor: None,
		state: None,
		state_null_flavor: None,
		postcode: None,
		postcode_null_flavor: None,
		telephone: None,
		telephone_null_flavor: None,
		country_code: None,
		email: None,
		email_null_flavor: None,
		qualification: None,
		qualification_null_flavor: None,
		qualification_kr1: None,
		primary_source_regulatory: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn sender() -> SenderInformation {
	SenderInformation {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		source_sender_presave_id: None,
		sender_type: None,
		health_professional_type_kr1: None,
		organization_name: None,
		department: None,
		street_address: None,
		city: None,
		state: None,
		postcode: None,
		country_code: None,
		person_title: None,
		person_title_null_flavor: None,
		person_given_name: None,
		person_given_name_null_flavor: None,
		person_middle_name: None,
		person_middle_name_null_flavor: None,
		person_family_name: None,
		person_family_name_null_flavor: None,
		telephone: None,
		fax: None,
		email: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn literature_reference(reference_text: String) -> LiteratureReference {
	LiteratureReference {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		reference_text: Some(reference_text),
		reference_text_null_flavor: None,
		sequence_number: 0,
		document_base64: None,
		file_name: None,
		media_type: None,
		representation: None,
		compression: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn study_registration(
	study_information_id: Uuid,
	registration_number: String,
	country_code: Option<String>,
	sequence_number: i32,
) -> StudyRegistrationNumber {
	StudyRegistrationNumber {
		id: Uuid::nil(),
		study_information_id,
		registration_number: Some(registration_number),
		registration_number_null_flavor: None,
		country_code,
		country_code_null_flavor: None,
		sequence_number,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn study(
	study_type_reaction: Option<&str>,
	sponsor_study_number: Option<&str>,
) -> StudyInformation {
	StudyInformation {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		source_study_presave_id: None,
		study_name: None,
		study_name_null_flavor: None,
		sponsor_study_number: sponsor_study_number.map(str::to_string),
		sponsor_study_number_null_flavor: None,
		study_type_reaction: study_type_reaction.map(str::to_string),
		study_type_reaction_kr1: None,
		fda_ind_number_occurred: None,
		fda_pre_anda_number_occurred: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn study_report() -> SafetyReportIdentification {
	let mut report = base_report();
	report.report_type = Some("2".to_string());
	report
}

pub(super) fn fda_context(
	cross_reported_inds: Vec<StudyFdaCrossReportedInd>,
	has_prior_submission: bool,
) -> FdaValidationContext {
	FdaValidationContext {
		studies: Vec::new(),
		cross_reported_inds,
		has_prior_submission,
	}
}

pub(super) fn cross_reported_ind(value: Option<&str>) -> StudyFdaCrossReportedInd {
	StudyFdaCrossReportedInd {
		id: Uuid::nil(),
		study_information_id: Uuid::nil(),
		ind_number: value.map(str::to_string),
		ind_number_null_flavor: None,
		sequence_number: 1,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn patient(initials: &str) -> PatientInformation {
	PatientInformation {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		patient_initials: Some(initials.to_string()),
		birth_date: None,
		age_at_time_of_onset: None,
		age_unit: None,
		gestation_period: None,
		gestation_period_unit: None,
		age_group: None,
		weight_kg: None,
		height_cm: None,
		sex: None,
		patient_initials_null_flavor: None,
		birth_date_null_flavor: None,
		sex_null_flavor: None,
		race_codes: Vec::new(),
		race_code_null_flavor: None,
		ethnicity_code: None,
		ethnicity_code_null_flavor: None,
		last_menstrual_period_date: None,
		last_menstrual_period_date_null_flavor: None,
		medical_history_text: None,
		medical_history_text_null_flavor: None,
		concomitant_therapy: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(in crate::case::sections) fn message_header(
	batch: &str,
	receiver: &str,
) -> MessageHeader {
	MessageHeader {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		batch_number: Some("batch".to_string()),
		batch_sender_identifier: Some("sender".to_string()),
		batch_receiver_identifier: Some(batch.to_string()),
		batch_transmission_date: None,
		message_type: "ichicsr".to_string(),
		message_format_version: "2.1".to_string(),
		message_format_release: "2.0".to_string(),
		message_number: "US-SENDER-1".to_string(),
		message_sender_identifier: "sender".to_string(),
		message_receiver_identifier: receiver.to_string(),
		message_date_format: "204".to_string(),
		message_date: "20200101000000".to_string(),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

#[test]
fn c_1_1_accepts_free_text_within_length() {
	let mut report = base_report();
	report.safety_report_id = Some("ICH-RICH-20260813-001".to_string());
	let mut issues = Vec::new();
	c_1_1(&report, &mut issues);
	assert!(issues.is_empty());
}

#[test]
fn primary_marker_is_semantically_checked() {
	let mut first = primary_source();
	first.primary_source_regulatory = Some("1".to_string());
	let mut second = primary_source();
	second.primary_source_regulatory = Some("1".to_string());
	let mut issues = Vec::new();
	c_2_r_5(&[first, second], &mut issues);

	assert!(issues
		.iter()
		.any(|issue| issue.code == "ICH.C.2.r.5.EXACTLY_ONCE"));
}

#[test]
fn fda_true_repeating_flags_require_a_child_row() {
	let mut report = base_report();
	report.additional_documents_available = Some(true);
	report.other_case_identifiers_exist = Some(true);
	let ctx = ctx_with(report);
	let mut issues = Vec::new();

	fda_repeating_flag_rules(&ctx, &mut issues);

	assert!(issues.iter().any(|issue| issue.code == "FDA.R0009"));
	assert!(issues.iter().any(|issue| issue.code == "FDA.R0017"));
}

#[test]
fn fda_initial_and_local_criteria_rules_reject_invalid_values() {
	let mut report = base_report();
	report.version = 99;
	report.fulfil_expedited_criteria_null_flavor = Some("NI".to_string());
	report.nullification_code = Some("1".to_string());
	report.local_criteria_report_type = Some("2".to_string());
	report.fulfil_expedited_criteria = Some(true);
	let header = message_header(crate::FDA_BATCH_RECEIVER_POSTMARKET, "CDER");
	let mut issues = Vec::new();

	fda_initial_report_rules(&report, Some(&header), false, &mut issues);
	fda_local_criteria_report_type(&report, Some(&header), &mut issues);

	for code in ["FDA.R0011", "FDA.R0101", "FDA.R0014"] {
		assert!(issues.iter().any(|issue| issue.code == code), "{code}");
	}
}

#[test]
fn fda_initial_rules_use_persisted_history_and_exact_route_scope() {
	let mut report = base_report();
	report.fulfil_expedited_criteria_null_flavor = Some("NI".to_string());
	report.nullification_code = Some("1".to_string());
	let premarket = message_header(
		crate::FDA_BATCH_RECEIVER_PREMARKET,
		crate::FDA_MSG_RECEIVER_CDER_IND,
	);
	let mut issues = Vec::new();

	fda_initial_report_rules(&report, Some(&premarket), false, &mut issues);
	assert!(issues.iter().any(|issue| issue.code == "FDA.R0011"));
	assert!(!issues.iter().any(|issue| issue.code == "FDA.R0101"));

	issues.clear();
	fda_initial_report_rules(&report, Some(&premarket), true, &mut issues);
	assert!(issues.is_empty());
}

#[test]
fn fda_primary_reporter_and_sender_fields_are_required() {
	let mut source = primary_source();
	source.primary_source_regulatory = Some("1".to_string());
	let mut issues = Vec::new();

	fda_primary_reporter_rules(&[source], true, &mut issues);
	fda_sender_rules(None, &mut issues);

	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.C.2.PRIMARY.REQUIRED"));
	assert!(issues.iter().any(|issue| issue.code == "FDA.R0020"));
	assert_eq!(
		issues
			.iter()
			.filter(|issue| issue.code == "FDA.C.3.SENDER.REQUIRED")
			.count(),
		12
	);
}

#[test]
fn fda_vaers_reporter_rules_only_apply_to_primary_us_reporter() {
	let mut source = primary_source();
	source.primary_source_regulatory = Some("1".to_string());
	source.country_code = Some("CA".to_string());
	source.reporter_given_name_null_flavor = Some("MSK".to_string());
	source.qualification = Some("1".to_string());
	let mut issues = Vec::new();

	fda_primary_reporter_rules(&[source.clone()], false, &mut issues);
	assert!(!issues.iter().any(|issue| {
		issue.code == "FDA.C.2.PRIMARY.REQUIRED"
			|| issue.code == "FDA.C.2.PRIMARY.MSK.FORBIDDEN"
	}));

	issues.clear();
	fda_primary_reporter_rules(&[source], true, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.C.2.PRIMARY.REQUIRED"));
	assert!(!issues
		.iter()
		.any(|issue| issue.code == "FDA.C.2.PRIMARY.MSK.FORBIDDEN"));
}

#[test]
fn fda_study_route_requires_valid_ind_and_cross_report() {
	let mut report = base_report();
	report.report_type = Some("2".to_string());
	let mut ctx = ctx_with(report);
	ctx.message_header = Some(message_header(
		crate::FDA_BATCH_RECEIVER_PREMARKET,
		crate::FDA_MSG_RECEIVER_CDER_IND,
	));
	let mut row = study(Some("1"), Some("SPONSOR"));
	row.fda_ind_number_occurred = Some("123".to_string());
	ctx.studies = vec![row];
	let mut issues = Vec::new();

	fda_study_route_rules(&ctx, &fda_context(Vec::new(), false), &mut issues);

	assert!(issues.iter().any(|issue| issue.code == "FDA.R0024.FORMAT"));
	assert!(issues.iter().any(|issue| issue.code == "FDA.R0026"));

	issues.clear();
	fda_study_route_rules(
		&ctx,
		&fda_context(vec![cross_reported_ind(Some("654321"))], false),
		&mut issues,
	);
	assert!(!issues.iter().any(|issue| issue.code == "FDA.R0026"));
}

#[test]
fn fda_aggregate_report_requires_study_type_one() {
	let mut ctx = ctx_with(base_report());
	ctx.message_header = Some(message_header(
		crate::FDA_BATCH_RECEIVER_PREMARKET,
		crate::FDA_MSG_RECEIVER_CDER_IND,
	));
	ctx.patient = Some(patient("AGGREGATE"));
	ctx.studies = vec![study(Some("2"), Some("SPONSOR"))];
	let mut issues = Vec::new();

	fda_study_route_rules(&ctx, &fda_context(Vec::new(), false), &mut issues);

	assert!(issues.iter().any(|issue| issue.code == "FDA.W0002"));
	assert!(issues.iter().any(|issue| issue.code == "FDA.W0001"));
}

#[test]
fn mfds_ct_receiver_enforces_reporter_sender_and_study_conditions() {
	let mut report = base_report();
	report.report_type = Some("1".to_string());
	report.fulfil_expedited_criteria = Some(false);
	let mut ctx = ctx_with(report);
	ctx.message_header = Some(message_header("MFDS-O-CT", "MFDS-O-CT"));
	ctx.primary_sources = vec![primary_source()];
	let mut issues = Vec::new();

	mfds_receiver_rules(&ctx, &mut issues);

	for code in [
		"MFDS.C.1.3.RECEIVER",
		"MFDS.C.1.7.RECEIVER",
		"MFDS.C.2.r.4.REQUIRED",
		"MFDS.C.2.RECEIVER.REQUIRED",
		"MFDS.C.2.r.2.3-5.RECEIVER.REQUIRED",
		"MFDS.C.3.3.3.RECEIVER.REQUIRED",
		"MFDS.C.5.1.r.1.RECEIVER.REQUIRED",
		"MFDS.C.5.RECEIVER.REQUIRED",
	] {
		assert!(issues.iter().any(|issue| issue.code == code), "{code}");
	}
}

#[test]
fn mfds_c_1_7_ni_requires_verified_r2_origin() {
	let mut report = base_report();
	report.fulfil_expedited_criteria_null_flavor = Some("NI".to_string());
	let mut ctx = ctx_with(report);
	ctx.message_header = Some(message_header("MFDS-O-KR", "MFDS-O-KR"));
	let mut issues = Vec::new();

	mfds_receiver_rules(&ctx, &mut issues);

	assert!(issues.iter().any(|issue| {
		issue.code == "MFDS.C.1.7.NULLFLAVOR.R2.RETRANSMISSION.REQUIRED"
	}));
}

#[test]
fn mfds_cu_allows_qualification_and_c_5_2_null_flavors() {
	let mut report = study_report();
	report.fulfil_expedited_criteria = Some(true);
	let mut ctx = ctx_with(report);
	ctx.message_header = Some(message_header("MFDS-O-CU", "MFDS-O-CU"));

	let mut source = primary_source();
	source.qualification_null_flavor = Some("UNK".to_string());
	ctx.primary_sources = vec![source];

	let mut study = study(Some("2"), Some("SPONSOR"));
	study.study_name_null_flavor = Some("ASKU".to_string());
	ctx.studies = vec![study];
	ctx.study_registrations = vec![study_registration(
		Uuid::nil(),
		"MFDS-APPROVAL".to_string(),
		Some("KR".to_string()),
		1,
	)];
	let mut issues = Vec::new();

	mfds_receiver_rules(&ctx, &mut issues);

	assert!(!issues
		.iter()
		.any(|issue| issue.code == "MFDS.C.2.r.4.NULLFLAVOR.FORBIDDEN.CT"));
	assert!(!issues.iter().any(|issue| {
		issue.code == "MFDS.C.5.RECEIVER.REQUIRED"
			&& issue.path == "studyInformation.0.studyName"
	}));
}

#[test]
fn mfds_kr1_extensions_only_apply_to_domestic_receiver() {
	let mut source = primary_source();
	source.qualification = Some("3".to_string());
	let mut sender = sender();
	sender.sender_type = Some("3".to_string());
	let study = study(Some("3"), Some("SPONSOR"));
	let mfds_ctx = MfdsValidationContext {
		senders: vec![sender],
		studies: vec![study],
		active_substances: Vec::new(),
		relatedness: Vec::new(),
		past_drugs: Vec::new(),
		parent_past_drugs: Vec::new(),
	};
	let mut ctx = ctx_with(base_report());
	ctx.primary_sources = vec![source];
	ctx.message_header = Some(message_header("MFDS-O-FR", "MFDS-O-FR"));
	let mut issues = Vec::new();

	collect_mfds_issues(&ctx, &mfds_ctx, &mut issues);
	assert!(!issues
		.iter()
		.any(|issue| issue.code.ends_with("KR.1.REQUIRED")));

	ctx.message_header = Some(message_header("MFDS-O-KR", "MFDS-O-KR"));
	issues.clear();
	collect_mfds_issues(&ctx, &mfds_ctx, &mut issues);
	for code in [
		"MFDS.C.2.r.4.KR.1.REQUIRED",
		"MFDS.C.3.1.KR.1.REQUIRED",
		"MFDS.C.5.4.KR.1.REQUIRED",
	] {
		assert!(issues.iter().any(|issue| issue.code == code), "{code}");
	}
}

#[test]
fn ich_requires_core_c_fields() {
	let ctx = ctx_with(base_report());
	let mut issues = Vec::new();
	collect_ich_issues(&ctx, &mut issues);
	for code in [
		"ICH.C.1.6.1.REQUIRED",
		"ICH.C.1.8.1.REQUIRED",
		"ICH.C.1.8.2.REQUIRED",
	] {
		assert!(issues.iter().any(|issue| issue.code == code), "{code}");
	}
}

#[test]
fn c_1_1_required_has_both_edges() {
	let mut report = base_report();
	let mut issues = Vec::new();
	c_1_1(&report, &mut issues);
	assert!(issues.iter().any(|issue| {
		issue.code == "ICH.C.1.1.REQUIRED"
			&& issue.path == "safetyReportIdentification.safetyReportId"
	}));

	issues.clear();
	report.safety_report_id = Some("CASE-1".to_string());
	c_1_1(&report, &mut issues);
	assert!(!issues
		.iter()
		.any(|issue| issue.code == "ICH.C.1.1.REQUIRED"));
}

#[test]
fn all_missing_flags_every_value_rule() {
	assert_eq!(
		snapshot(base_report()),
		vec![
			issue(
				"ICH.C.1.2.REQUIRED",
				"safetyReportIdentification.transmissionDate",
			),
			issue(
				"ICH.C.1.3.REQUIRED",
				"safetyReportIdentification.reportType",
			),
			issue(
				"ICH.C.1.4.REQUIRED",
				"safetyReportIdentification.dateFirstReceivedFromSource",
			),
			issue(
				"ICH.C.1.5.REQUIRED",
				"safetyReportIdentification.dateOfMostRecentInformation",
			),
			issue(
				"ICH.C.1.7.REQUIRED",
				"safetyReportIdentification.fulfilExpeditedCriteria",
			),
		]
	);
}

#[test]
fn all_present_flags_nothing() {
	let mut report = base_report();
	report.safety_report_id = Some("US-ABC-1".to_string());
	report.transmission_date = Some("20200101120000".to_string());
	report.report_type = Some("1".to_string());
	report.date_first_received_from_source = Some("20200101".to_string());
	report.date_of_most_recent_information = Some("20200101".to_string());
	report.fulfil_expedited_criteria = Some(true);
	assert_eq!(snapshot(report), Vec::new());
}

#[test]
fn c1_information_dates_may_follow_creation_date() {
	let mut report = base_report();
	report.transmission_date = Some("20140714151617-0500".to_string());
	report.date_first_received_from_source = Some("20220614".to_string());
	report.date_of_most_recent_information = Some("20220614".to_string());
	let mut issues = Vec::new();

	c_1_4(&report, &mut issues);
	c_1_5(&report, &mut issues);

	assert!(issues.is_empty(), "{issues:?}");
}

#[test]
fn c1_7_nullflavor_without_r2_provenance_fails_closed() {
	let mut report = base_report();
	report.fulfil_expedited_criteria = None;
	report.fulfil_expedited_criteria_null_flavor = Some("NI".to_string());
	let snap = snapshot(report);
	assert!(snap.iter().any(|(code, _)| code == "ICH.C.1.7.REQUIRED"));
}

#[test]
fn indexed_document_missing_title_flags_matching_index() {
	let mut ctx = ctx_with(base_report());
	ctx.documents_held_by_sender = vec![document(Some("attached")), document(None)];
	assert_eq!(
		filtered(&ctx, INDEXED_CODES),
		vec![issue(
			"ICH.C.1.6.1.r.1.REQUIRED",
			"documentsHeldBySender.1.documentDescription",
		)]
	);
}

#[test]
fn indexed_other_identifiers_flag_empty_fields_per_index() {
	let mut ctx = ctx_with(base_report());
	ctx.other_case_identifiers =
		vec![other_identifier("SRC", "ID"), other_identifier("", "")];
	assert_eq!(
		filtered(&ctx, INDEXED_CODES),
		vec![
			issue("ICH.C.1.9.1.r.1.REQUIRED", "otherCaseIdentifiers.1.source",),
			issue(
				"ICH.C.1.9.1.r.2.REQUIRED",
				"otherCaseIdentifiers.1.caseIdentifier",
			),
		]
	);
}

#[test]
fn c1_9_1_r_2_validates_country_sender_report_format() {
	let vocabulary = crate::context::VocabularyContext::for_active_codes(&[
		("ISO3166", crate::VocabularyScope::All, "EU"),
		("ISO3166", crate::VocabularyScope::All, "KR"),
	]);
	let identifier = other_identifier("Legacy Safety DB", "EU-ORG-2026-4451");
	let mut issues = Vec::new();

	c_1_9_1_r_2(0, &identifier, &vocabulary, &mut issues);

	assert!(issues.is_empty(), "{issues:?}");

	let mut ctx = ctx_with(base_report());
	ctx.vocabulary = vocabulary.clone();
	ctx.other_case_identifiers = vec![
		other_identifier("Prior sender", "KR-ORG-127524"),
		other_identifier("Prior sender", "bad-ee1b6e"),
	];
	collect_ich_issues(&ctx, &mut issues);
	assert_eq!(
		issues
			.iter()
			.filter(|issue| issue.code == "ICH.C.1.9.1.r.2.FORMAT")
			.map(|issue| issue.path.as_str())
			.collect::<Vec<_>>(),
		vec!["otherCaseIdentifiers.1.caseIdentifier"]
	);

	issues.clear();
	for value in [
		"KR--127524",
		"KR-ORG",
		"ZZ-ORG-127524",
		"-ORG-123",
		"KR- -123",
		"KR-ORG-   ",
		"K R-ORG-123",
	] {
		c_1_9_1_r_2(
			0,
			&other_identifier("Prior sender", value),
			&vocabulary,
			&mut issues,
		);
	}
	assert_eq!(
		issues
			.iter()
			.filter(|issue| issue.code == "ICH.C.1.9.1.r.2.FORMAT")
			.count(),
		7
	);
}

#[test]
fn ci_repeating_issues_use_editor_field_names() {
	let mut ctx = ctx_with(base_report());
	let mut document = document(None);
	document.document_base64 = Some("not-base64".to_string());
	ctx.documents_held_by_sender = vec![document];
	ctx.other_case_identifiers = vec![other_identifier("", "")];
	ctx.linked_report_numbers = vec![linked_report_number("L".repeat(101))];

	assert_eq!(
		filtered(
			&ctx,
			&[
				"ICH.C.1.6.1.r.1.REQUIRED",
				"ICH.C.1.6.1.r.2.ALLOWED.VALUE",
				"ICH.C.1.9.1.r.1.REQUIRED",
				"ICH.C.1.9.1.r.2.REQUIRED",
				"ICH.C.1.10.r.LENGTH.MAX",
			],
		),
		vec![
			issue(
				"ICH.C.1.10.r.LENGTH.MAX",
				"linkedReports.0.linkedReportNumber",
			),
			issue(
				"ICH.C.1.6.1.r.1.REQUIRED",
				"documentsHeldBySender.0.documentDescription",
			),
			issue(
				"ICH.C.1.6.1.r.2.ALLOWED.VALUE",
				"documentsHeldBySender.0.includedDocument",
			),
			issue("ICH.C.1.9.1.r.1.REQUIRED", "otherCaseIdentifiers.0.source",),
			issue(
				"ICH.C.1.9.1.r.2.REQUIRED",
				"otherCaseIdentifiers.0.caseIdentifier",
			),
		]
	);
}

#[test]
fn fda_attachments_require_file_name_and_matching_media_type() {
	let mut document = document(Some("held"));
	document.document_base64 = Some("QUJD".to_string());
	let mut reference = literature_reference("paper".to_string());
	reference.document_base64 = Some("REVG".to_string());
	reference.file_name = Some("paper.pdf".to_string());
	reference.media_type = Some("text/plain".to_string());
	let mut issues = Vec::new();
	fda_c_1_6_1_r_2(0, &document, &mut issues);
	fda_c_4_r_2(0, &reference, &mut issues);
	let mut actual = issues
		.into_iter()
		.map(|issue| (issue.code, issue.path))
		.collect::<Vec<_>>();
	actual.sort();
	assert_eq!(
		actual,
		vec![
			issue(
				"FDA.C.1.6.1.r.2.FILE_NAME.REQUIRED",
				"documentsHeldBySender.0.includedDocument",
			),
			issue(
				"FDA.C.4.r.2.MEDIA_TYPE.MATCH",
				"literatureReferences.0.documentBase64",
			),
		]
	);
}

#[test]
fn study_rules_flag_missing_fields_when_study_report() {
	let mut ctx = ctx_with(study_report());
	ctx.studies = vec![study(None, None)];
	assert_eq!(
		filtered(&ctx, STUDY_CODES),
		vec![issue(
			"ICH.C.5.4.REQUIRED",
			"studyInformation.0.studyTypeReaction",
		)]
	);
}

#[test]
fn study_rules_silent_when_not_study_report() {
	// report_type != "2" gates the whole study block off.
	let mut ctx = ctx_with(base_report());
	ctx.studies = vec![study(None, None)];
	assert_eq!(filtered(&ctx, STUDY_CODES), Vec::new());
}

#[test]
fn study_rules_pass_when_fields_present() {
	let mut ctx = ctx_with(study_report());
	ctx.studies = vec![study(Some("1"), Some("SPONSOR-1"))];
	assert_eq!(filtered(&ctx, STUDY_CODES), Vec::new());
}

#[test]
fn sponsor_study_number_is_optional_with_or_without_null_flavor() {
	for null_flavor in [None, Some("ASKU"), Some("NASK")] {
		let mut study = study(Some("1"), None);
		study.sponsor_study_number_null_flavor = null_flavor.map(str::to_string);
		let mut ctx = ctx_with(study_report());
		ctx.studies = vec![study];

		assert_eq!(filtered(&ctx, STUDY_CODES), Vec::new());
	}
}

#[test]
fn study_reporter_organization_remains_optional() {
	let mut source = primary_source();
	source.organization_null_flavor = Some("NASK".to_string());
	let mut ctx = ctx_with(study_report());
	ctx.primary_sources = vec![source];

	let mut issues = Vec::new();
	collect_ich_issues(&ctx, &mut issues);
	assert!(!issues
		.iter()
		.any(|issue| issue.path == "primarySources.0.reporterOrganization"));
}

#[test]
fn fda_reporter_email_rule_applies_to_every_row_outside_vaers_rules() {
	let mut issues = Vec::new();
	let primary = primary_source();
	let mut secondary = primary_source();
	secondary.primary_source_regulatory = Some("2".to_string());
	fda_reporter_email_rules(&[primary, secondary], &mut issues);
	let mut out = issues
		.into_iter()
		.filter(|issue| issue.code == "FDA.C.2.r.2.8.REQUIRED")
		.map(|issue| (issue.code, issue.path))
		.collect::<Vec<_>>();
	out.sort();

	assert_eq!(
		out,
		vec![
			issue("FDA.C.2.r.2.8.REQUIRED", "primarySources.0.reporterEmail"),
			issue("FDA.C.2.r.2.8.REQUIRED", "primarySources.1.reporterEmail"),
		]
	);
}

#[test]
fn fda_reporter_email_requires_a_non_blank_value_or_null_flavor() {
	let mut source = primary_source();
	source.email_null_flavor = Some("   ".to_string());
	let mut issues = Vec::new();

	fda_reporter_email_rules(&[source], &mut issues);

	assert_eq!(
		issues
			.into_iter()
			.filter(|issue| issue.code == "FDA.C.2.r.2.8.REQUIRED")
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>(),
		vec![issue(
			"FDA.C.2.r.2.8.REQUIRED",
			"primarySources.0.reporterEmail",
		)]
	);
}

#[test]
fn allowed_value_rule_flags_invalid_report_type() {
	let mut report = base_report();
	report.report_type = Some("9".to_string());
	let ctx = ctx_with(report);

	assert_eq!(
		filtered(&ctx, &["ICH.C.1.3.ALLOWED.VALUE"]),
		vec![issue(
			"ICH.C.1.3.ALLOWED.VALUE",
			"safetyReportIdentification.reportType",
		)]
	);
}

#[test]
fn datetime_format_rule_flags_invalid_transmission_date() {
	let mut report = base_report();
	report.transmission_date = Some("not-a-date".to_string());
	let ctx = ctx_with(report);

	assert_eq!(
		filtered(&ctx, &["ICH.C.1.2.ALLOWED.VALUE"]),
		vec![issue(
			"ICH.C.1.2.ALLOWED.VALUE",
			"safetyReportIdentification.transmissionDate",
		)]
	);
}

#[test]
fn allowed_value_rules_cover_c_sender_source_and_study_codes() {
	let mut report = base_report();
	report.first_sender_type = Some("9".to_string());
	report.other_case_identifiers_exist = Some(false);
	report.nullification_code = Some("9".to_string());
	let mut ctx = ctx_with(report);

	let mut source = primary_source();
	source.qualification = Some("9".to_string());
	source.primary_source_regulatory = Some("9".to_string());
	ctx.primary_sources = vec![source];

	let mut sender = sender();
	sender.sender_type = Some("9".to_string());
	ctx.sender = Some(sender);

	ctx.studies = vec![study(Some("9"), Some("SPONSOR-1"))];

	assert_eq!(
		filtered(
			&ctx,
			&[
				"ICH.C.1.8.2.ALLOWED.VALUE",
				"ICH.C.1.9.1.ALLOWED.VALUE",
				"ICH.C.1.11.1.ALLOWED.VALUE",
				"ICH.C.2.r.4.ALLOWED.VALUE",
				"ICH.C.2.r.5.ALLOWED.VALUE",
				"ICH.C.3.1.ALLOWED.VALUE",
				"ICH.C.5.4.ALLOWED.VALUE",
			],
		),
		vec![
			issue(
				"ICH.C.1.11.1.ALLOWED.VALUE",
				"safetyReportIdentification.nullificationAmendmentCode",
			),
			issue(
				"ICH.C.1.8.2.ALLOWED.VALUE",
				"safetyReportIdentification.firstSenderType",
			),
			issue(
				"ICH.C.1.9.1.ALLOWED.VALUE",
				"safetyReportIdentification.otherCaseIdentifiersExist",
			),
			issue(
				"ICH.C.2.r.4.ALLOWED.VALUE",
				"primarySources.0.qualification",
			),
			issue(
				"ICH.C.2.r.5.ALLOWED.VALUE",
				"primarySources.0.primarySourceForRegulatoryPurposes",
			),
			issue("ICH.C.3.1.ALLOWED.VALUE", "senderInformation.senderType",),
			issue(
				"ICH.C.5.4.ALLOWED.VALUE",
				"studyInformation.0.studyTypeReaction",
			),
		]
	);
}

#[test]
fn true_marker_allows_dictionary_null_flavor() {
	let mut report = base_report();
	report.other_case_identifiers_exist = Some(false);
	report.other_case_identifiers_exist_null_flavor = Some("NI".to_string());
	let ctx = ctx_with(report);

	assert_eq!(filtered(&ctx, &["ICH.C.1.9.1.ALLOWED.VALUE"]), Vec::new());
}

#[test]
fn max_length_rules_cover_c1_and_indexed_fields() {
	let mut report = base_report();
	report.safety_report_id = Some("S".repeat(101));
	report.report_type = Some("22".to_string());
	report.worldwide_unique_id = Some("W".repeat(101));
	report.first_sender_type = Some("12".to_string());
	report.nullification_code = Some("12".to_string());
	report.nullification_reason = Some("R".repeat(2001));
	let mut ctx = ctx_with(report);
	ctx.documents_held_by_sender = vec![document(Some(&"D".repeat(2001)))];
	ctx.other_case_identifiers =
		vec![other_identifier(&"S".repeat(101), &"I".repeat(101))];
	ctx.linked_report_numbers = vec![linked_report_number("L".repeat(101))];
	ctx.studies = vec![study(Some("12"), Some(&"N".repeat(51)))];

	assert_eq!(
		filtered(&ctx, LENGTH_CODES),
		vec![
			issue(
				"ICH.C.1.1.LENGTH.MAX",
				"safetyReportIdentification.safetyReportId",
			),
			issue(
				"ICH.C.1.10.r.LENGTH.MAX",
				"linkedReports.0.linkedReportNumber",
			),
			issue(
				"ICH.C.1.11.1.LENGTH.MAX",
				"safetyReportIdentification.nullificationAmendmentCode",
			),
			issue(
				"ICH.C.1.11.2.LENGTH.MAX",
				"safetyReportIdentification.nullificationReason",
			),
			issue(
				"ICH.C.1.3.LENGTH.MAX",
				"safetyReportIdentification.reportType",
			),
			issue(
				"ICH.C.1.6.1.r.1.LENGTH.MAX",
				"documentsHeldBySender.0.documentDescription",
			),
			issue(
				"ICH.C.1.8.1.LENGTH.MAX",
				"safetyReportIdentification.worldwideUniqueId",
			),
			issue(
				"ICH.C.1.8.2.LENGTH.MAX",
				"safetyReportIdentification.firstSenderType",
			),
			issue(
				"ICH.C.1.9.1.r.1.LENGTH.MAX",
				"otherCaseIdentifiers.0.source",
			),
			issue(
				"ICH.C.1.9.1.r.2.LENGTH.MAX",
				"otherCaseIdentifiers.0.caseIdentifier",
			),
			issue(
				"ICH.C.5.3.LENGTH.MAX",
				"studyInformation.0.sponsorStudyNumber",
			),
			issue(
				"ICH.C.5.4.LENGTH.MAX",
				"studyInformation.0.studyTypeReaction",
			),
		]
	);
}

#[test]
fn max_length_rules_cover_c2_c3_and_study_name_fields() {
	let mut source = primary_source();
	source.reporter_title = Some("T".repeat(51));
	source.reporter_given_name = Some("G".repeat(61));
	source.reporter_middle_name = Some("M".repeat(61));
	source.reporter_family_name = Some("F".repeat(61));
	source.organization = Some("O".repeat(61));
	source.department = Some("D".repeat(61));
	source.street = Some("S".repeat(101));
	source.city = Some("C".repeat(36));
	source.state = Some("S".repeat(41));
	source.postcode = Some("P".repeat(16));
	source.telephone = Some("T".repeat(34));
	source.country_code = Some("USA".to_string());
	source.qualification = Some("12".to_string());
	source.primary_source_regulatory = Some("12".to_string());

	let mut sender = sender();
	sender.sender_type = Some("12".to_string());
	sender.organization_name = Some("O".repeat(101));
	sender.department = Some("D".repeat(61));
	sender.person_title = Some("T".repeat(51));
	sender.person_given_name = Some("G".repeat(61));
	sender.person_middle_name = Some("M".repeat(61));
	sender.person_family_name = Some("F".repeat(61));
	sender.street_address = Some("S".repeat(101));
	sender.city = Some("C".repeat(36));
	sender.state = Some("S".repeat(41));
	sender.postcode = Some("P".repeat(16));
	sender.country_code = Some("USA".to_string());
	sender.telephone = Some("T".repeat(34));
	sender.fax = Some("F".repeat(34));
	sender.email = Some("E".repeat(101));

	let mut study = study(Some("1"), Some("SPONSOR-1"));
	study.study_name = Some("S".repeat(2001));

	let mut ctx = ctx_with(base_report());
	ctx.primary_sources = vec![source];
	ctx.sender = Some(sender);
	ctx.studies = vec![study];

	assert_eq!(
		filtered(&ctx, C23_LENGTH_CODES),
		vec![
			issue("ICH.C.2.r.1.1.LENGTH.MAX", "primarySources.0.reporterTitle",),
			issue(
				"ICH.C.2.r.1.2.LENGTH.MAX",
				"primarySources.0.reporterGivenName",
			),
			issue(
				"ICH.C.2.r.1.3.LENGTH.MAX",
				"primarySources.0.reporterMiddleName",
			),
			issue(
				"ICH.C.2.r.1.4.LENGTH.MAX",
				"primarySources.0.reporterFamilyName",
			),
			issue(
				"ICH.C.2.r.2.1.LENGTH.MAX",
				"primarySources.0.reporterOrganization",
			),
			issue(
				"ICH.C.2.r.2.2.LENGTH.MAX",
				"primarySources.0.reporterDepartment",
			),
			issue(
				"ICH.C.2.r.2.3.LENGTH.MAX",
				"primarySources.0.reporterStreet",
			),
			issue("ICH.C.2.r.2.4.LENGTH.MAX", "primarySources.0.reporterCity",),
			issue("ICH.C.2.r.2.5.LENGTH.MAX", "primarySources.0.reporterState",),
			issue(
				"ICH.C.2.r.2.6.LENGTH.MAX",
				"primarySources.0.reporterPostcode",
			),
			issue(
				"ICH.C.2.r.2.7.LENGTH.MAX",
				"primarySources.0.reporterTelephone",
			),
			issue("ICH.C.2.r.3.LENGTH.MAX", "primarySources.0.reporterCountry",),
			issue("ICH.C.2.r.4.LENGTH.MAX", "primarySources.0.qualification",),
			issue(
				"ICH.C.2.r.5.LENGTH.MAX",
				"primarySources.0.primarySourceForRegulatoryPurposes",
			),
			issue("ICH.C.3.1.LENGTH.MAX", "senderInformation.senderType",),
			issue("ICH.C.3.2.LENGTH.MAX", "senderInformation.organizationName",),
			issue("ICH.C.3.3.1.LENGTH.MAX", "senderInformation.department",),
			issue("ICH.C.3.3.2.LENGTH.MAX", "senderInformation.personTitle",),
			issue(
				"ICH.C.3.3.3.LENGTH.MAX",
				"senderInformation.personGivenName",
			),
			issue(
				"ICH.C.3.3.4.LENGTH.MAX",
				"senderInformation.personMiddleName",
			),
			issue(
				"ICH.C.3.3.5.LENGTH.MAX",
				"senderInformation.personFamilyName",
			),
			issue("ICH.C.3.4.1.LENGTH.MAX", "senderInformation.streetAddress",),
			issue("ICH.C.3.4.2.LENGTH.MAX", "senderInformation.city",),
			issue("ICH.C.3.4.3.LENGTH.MAX", "senderInformation.state",),
			issue("ICH.C.3.4.4.LENGTH.MAX", "senderInformation.postcode",),
			issue("ICH.C.3.4.5.LENGTH.MAX", "senderInformation.countryCode",),
			issue("ICH.C.3.4.6.LENGTH.MAX", "senderInformation.telephone",),
			issue("ICH.C.3.4.7.LENGTH.MAX", "senderInformation.fax",),
			issue("ICH.C.3.4.8.LENGTH.MAX", "senderInformation.email",),
			issue("ICH.C.5.2.LENGTH.MAX", "studyInformation.0.studyName",),
		],
	);
}

#[test]
fn max_length_rules_cover_literature_and_study_registration_fields() {
	let study_id = Uuid::nil();
	let mut study = study(Some("1"), Some("SPONSOR-1"));
	study.id = study_id;

	let mut ctx = ctx_with(base_report());
	ctx.literature_references = vec![literature_reference("R".repeat(501))];
	ctx.studies = vec![study];
	ctx.study_registrations = vec![study_registration(
		study_id,
		"N".repeat(51),
		Some("USA".to_string()),
		7, // Deleted earlier rows must not shift the editor error index.
	)];

	assert_eq!(
		filtered(&ctx, C45_LENGTH_CODES),
		vec![
			issue(
				"ICH.C.4.r.1.LENGTH.MAX",
				"literatureReferences.0.referenceText",
			),
			issue(
				"ICH.C.5.1.r.1.LENGTH.MAX",
				"studyInformation.0.registrations.0.registrationNumber",
			),
			issue(
				"ICH.C.5.1.r.2.LENGTH.MAX",
				"studyInformation.0.registrations.0.registrationCountry",
			),
		],
	);
}

#[test]
fn golden_c_issue_metadata() {
	let mut issues = Vec::new();
	let mut report = base_report();
	report.report_type = Some("9".to_string());
	c_1_3(&report, &mut issues);
	c_2_r_5(&[], &mut issues);
	c_3_2(None, &mut issues);
	c_5_4(&[], true, &mut issues);
	mfds_c_3_1_kr_1(2, None, true, &mut issues);

	let mut out = issues
		.into_iter()
		.filter(|issue| {
			matches!(
				issue.code.as_str(),
				"ICH.C.1.REQUIRED"
					| "ICH.C.1.3.ALLOWED.VALUE"
					| "ICH.C.2.r.REQUIRED"
					| "ICH.C.3.2.REQUIRED"
					| "ICH.C.5.4.REQUIRED"
					| "MFDS.C.3.1.KR.1.REQUIRED"
			)
		})
		.map(|issue| {
			(
				issue.code,
				issue.message,
				issue.path,
				issue.field_path,
				issue.section,
				issue.subsection,
			)
		})
		.collect::<Vec<_>>();
	out.sort_by(|left, right| left.0.cmp(&right.0));

	assert_eq!(
		out,
		vec![
			(
				"ICH.C.1.3.ALLOWED.VALUE".to_string(),
				"Dictionary allowed values constraint.".to_string(),
				"safetyReportIdentification.reportType".to_string(),
				Some("safetyReportIdentification.reportType".to_string()),
				"case-identification".to_string(),
				"C.1".to_string(),
			),
			(
				"ICH.C.2.r.REQUIRED".to_string(),
				"At least one reporter is required."
					.to_string(),
				"primarySources".to_string(),
				Some(
					"primarySources".to_string(),
				),
				"C".to_string(),
				"C.2".to_string(),
			),
			(
				"ICH.C.3.2.REQUIRED".to_string(),
				"[C.3.2] is required.".to_string(),
				"senderInformation.organizationName".to_string(),
				Some("senderInformation.organizationName".to_string()),
				"sender".to_string(),
				"C.3".to_string(),
			),
			(
				"ICH.C.5.4.REQUIRED".to_string(),
				"[C.5.4] Study type where reaction(s) / event(s) were observed is required when [C.1.3] is report from study (2).".to_string(),
				"studyInformation.0.studyTypeReaction".to_string(),
				Some("studyInformation.0.studyTypeReaction".to_string()),
				"study".to_string(),
				"C.5".to_string(),
			),
			(
				"MFDS.C.3.1.KR.1.REQUIRED".to_string(),
				"MFDS requires [C.3.1.KR.1] when sender type [C.3.1] is health professional (3).".to_string(),
				"senderInformation.2.healthProfessionalTypeKr1".to_string(),
				Some("senderInformation.2.healthProfessionalTypeKr1".to_string()),
				"case-identification".to_string(),
				"C.3".to_string(),
			),
		],
	);
}

#[test]
fn missing_reporter_is_one_list_error_but_existing_reporter_has_field_errors() {
	let mut ctx = ctx_with(study_report());
	let mut issues = Vec::new();
	collect_ich_issues(&ctx, &mut issues);
	let reporter: Vec<_> = issues
		.iter()
		.filter(|issue| issue.path.starts_with("primarySources"))
		.collect();
	assert_eq!(reporter.len(), 1, "{reporter:?}");
	assert_eq!(reporter[0].path, "primarySources");
	ctx.primary_sources = vec![primary_source()];
	ctx.primary_sources[0].qualification = None;
	ctx.primary_sources[0].primary_source_regulatory = Some("1".into());
	issues.clear();
	collect_ich_issues(&ctx, &mut issues);
	assert!(!issues.iter().any(|issue| issue.path == "primarySources"));
	assert!(issues
		.iter()
		.any(|issue| issue.path == "primarySources.0.qualification"));
}

#[test]
fn absent_conditional_lists_use_collection_paths() {
	let mut ctx = ctx_with(study_report());
	let report = ctx.safety_report.as_mut().unwrap();
	report.additional_documents_available = Some(true);
	report.other_case_identifiers_exist = Some(true);
	let mut issues = Vec::new();
	fda_repeating_flag_rules(&ctx, &mut issues);
	assert_eq!(
		issues
			.iter()
			.map(|issue| issue.path.as_str())
			.collect::<Vec<_>>(),
		vec!["documentsHeldBySender", "otherCaseIdentifiers"]
	);
	ctx.message_header = Some(message_header("MFDS-O-CT", "MFDS-O-CT"));
	issues.clear();
	mfds_receiver_rules(&ctx, &mut issues);
	let registration: Vec<_> = issues
		.iter()
		.filter(|issue| issue.path.contains("registrations"))
		.collect();
	assert_eq!(registration.len(), 1);
	assert_eq!(registration[0].path, "studyInformation.registrations");
}
