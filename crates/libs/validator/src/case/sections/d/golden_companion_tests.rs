//! Characterization tests for the MedDRA code⇔version companion rules in
//! `collect_ich_issues` (D.7.1.r.1a / D.7.1.r.1b on medical history). They
//! freeze current behavior (code + path) before the table-driven refactor.
//! Cross-field date rules (`*.FUTURE_DATE`) stay out of scope and inline.
use super::*;
use lib_core::model::case::Case;
use lib_core::model::parent_history::{ParentMedicalHistory, ParentPastDrugHistory};
use lib_core::model::patient::{
	MedicalHistoryEpisode, ParentInformation, PastDrugHistory,
	PatientDeathInformation, PatientIdentifier, PatientInformation,
};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Decimal;
use sqlx::types::Uuid;
use time::Month;

const MEDHIST_CODES: &[&str] =
	&["ICH.D.7.1.r.1a.REQUIRED", "ICH.D.7.1.r.1b.REQUIRED"];

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

pub(super) fn empty_ctx() -> ValidationContext {
	ValidationContext {
		vocabulary: Default::default(),
		case: dummy_case(),
		safety_report: None,
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

pub(super) fn medhist(
	meddra_code: Option<&str>,
	meddra_version: Option<&str>,
) -> MedicalHistoryEpisode {
	MedicalHistoryEpisode {
		id: Uuid::nil(),
		patient_id: Uuid::nil(),
		sequence_number: 0,
		meddra_version: meddra_version.map(str::to_string),
		meddra_code: meddra_code.map(str::to_string),
		start_date: None,
		start_date_null_flavor: None,
		continuing: None,
		continuing_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		comments: None,
		family_history: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn parent(id: Uuid) -> ParentInformation {
	ParentInformation {
		id,
		patient_id: Uuid::nil(),
		parent_identification: None,
		parent_identification_null_flavor: None,
		parent_birth_date: None,
		parent_birth_date_null_flavor: None,
		parent_age: None,
		parent_age_unit: None,
		last_menstrual_period_date: None,
		last_menstrual_period_date_null_flavor: None,
		weight_kg: None,
		height_cm: None,
		sex: None,
		sex_null_flavor: None,
		medical_history_text: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn patient() -> PatientInformation {
	PatientInformation {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		patient_initials: None,
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
		medical_history_text: Some("history".to_string()),
		medical_history_text_null_flavor: None,
		concomitant_therapy: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn patient_identifier(
	identifier_type_code: &str,
	identifier_value: &str,
) -> PatientIdentifier {
	PatientIdentifier {
		id: Uuid::nil(),
		patient_id: Uuid::nil(),
		sequence_number: 1,
		identifier_type_code: identifier_type_code.to_string(),
		identifier_value: Some(identifier_value.to_string()),
		identifier_value_null_flavor: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn past_drug() -> PastDrugHistory {
	PastDrugHistory {
		id: Uuid::nil(),
		patient_id: Uuid::nil(),
		sequence_number: 1,
		drug_name: None,
		drug_name_null_flavor: None,
		mfds_medicinal_product_version: None,
		mfds_medicinal_product_id: None,
		mpid: None,
		mpid_version: None,
		mpid_source_code_system: None,
		mpid_source_code_system_version: None,
		phpid: None,
		phpid_version: None,
		start_date: None,
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		indication_meddra_version: None,
		indication_meddra_code: None,
		reaction_meddra_version: None,
		reaction_meddra_code: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn death_info() -> PatientDeathInformation {
	PatientDeathInformation {
		id: Uuid::nil(),
		patient_id: Uuid::nil(),
		date_of_death: None,
		date_of_death_null_flavor: None,
		autopsy_performed: Some(false),
		autopsy_performed_null_flavor: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn parent_medhist(
	parent_id: Uuid,
	meddra_code: Option<&str>,
	meddra_version: Option<&str>,
) -> ParentMedicalHistory {
	ParentMedicalHistory {
		id: Uuid::nil(),
		parent_id,
		sequence_number: 1,
		meddra_version: meddra_version.map(str::to_string),
		meddra_code: meddra_code.map(str::to_string),
		start_date: None,
		start_date_null_flavor: None,
		continuing: None,
		continuing_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		comments: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn parent_past_drug(
	parent_id: Uuid,
	mpid: Option<&str>,
	mpid_version: Option<&str>,
) -> ParentPastDrugHistory {
	ParentPastDrugHistory {
		id: Uuid::nil(),
		parent_id,
		sequence_number: 1,
		drug_name: None,
		mpid: mpid.map(str::to_string),
		mpid_version: mpid_version.map(str::to_string),
		mfds_medicinal_product_version: None,
		mfds_medicinal_product_id: None,
		phpid: None,
		phpid_version: None,
		start_date: None,
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		indication_meddra_version: None,
		indication_meddra_code: None,
		reaction_meddra_version: None,
		reaction_meddra_code: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn medhist_codes(episode: MedicalHistoryEpisode) -> Vec<(String, String)> {
	let mut ctx = empty_ctx();
	ctx.medical_history = vec![episode];
	let mut issues = Vec::new();
	collect_ich_issues(&ctx, &mut issues);
	let mut out: Vec<(String, String)> = issues
		.into_iter()
		.filter(|issue| MEDHIST_CODES.contains(&issue.code.as_str()))
		.map(|issue| (issue.code, issue.path))
		.collect();
	out.sort();
	out
}

#[test]
fn code_without_version_flags_1a() {
	assert_eq!(
		medhist_codes(medhist(Some("10000001"), None)),
		vec![(
			"ICH.D.7.1.r.1a.REQUIRED".to_string(),
			"patientInformation.medicalHistory.0.meddraVersion".to_string()
		)]
	);
}

#[test]
fn version_without_code_flags_1b() {
	assert_eq!(
		medhist_codes(medhist(None, Some("27.0"))),
		vec![(
			"ICH.D.7.1.r.1b.REQUIRED".to_string(),
			"patientInformation.medicalHistory.0.meddraCode".to_string()
		)]
	);
}

#[test]
fn both_present_is_silent() {
	assert_eq!(
		medhist_codes(medhist(Some("10000001"), Some("27.0"))),
		Vec::new()
	);
}

#[test]
fn both_absent_is_silent() {
	assert_eq!(medhist_codes(medhist(None, None)), Vec::new());
}

#[test]
fn parent_child_companion_paths_use_the_owning_parent_index() {
	let first_parent_id = Uuid::from_u128(1);
	let second_parent_id = Uuid::from_u128(2);
	let mut ctx = empty_ctx();
	ctx.parents = vec![parent(first_parent_id), parent(second_parent_id)];
	ctx.parent_medical_history =
		vec![parent_medhist(second_parent_id, Some("10000001"), None)];
	ctx.parent_medical_history[0].sequence_number = 7;
	let mut exclusive_parent_past_drug =
		parent_past_drug(second_parent_id, Some("MPID"), Some("1"));
	exclusive_parent_past_drug.sequence_number = 9;
	exclusive_parent_past_drug.phpid = Some("PHPID".to_string());
	ctx.parent_past_drugs = vec![
		parent_past_drug(second_parent_id, Some("MPID"), None),
		exclusive_parent_past_drug,
	];
	ctx.parent_past_drugs[0].sequence_number = 4;

	let mut issues = Vec::new();
	collect_ich_issues(&ctx, &mut issues);
	let mut out: Vec<(String, String)> = issues
		.into_iter()
		.filter(|issue| {
			matches!(
				issue.code.as_str(),
				"ICH.D.10.7.1.r.1a.REQUIRED"
					| "ICH.D.10.8.r.2a.REQUIRED"
					| "ICH.D.10.8.MPID_PHPID.EXCLUSIVE"
			)
		})
		.map(|issue| (issue.code, issue.path))
		.collect();
	out.sort();

	assert_eq!(
		out,
		vec![
			(
				"ICH.D.10.7.1.r.1a.REQUIRED".to_string(),
				"patientInformation.parents.1.medicalHistory.0.meddraVersion"
					.to_string()
			),
			(
				"ICH.D.10.8.MPID_PHPID.EXCLUSIVE".to_string(),
				"patientInformation.parents.1.pastDrugs.1.mpid".to_string()
			),
			(
				"ICH.D.10.8.r.2a.REQUIRED".to_string(),
				"patientInformation.parents.1.pastDrugs.0.mpidVersion".to_string()
			),
		]
	);
}

const REPORTED_CAUSE_CODES: &[&str] = &[
	"ICH.D.9.2.r.1a.REQUIRED",
	"ICH.D.9.2.r.1b.REQUIRED",
	"ICH.D.9.2.r.2.REQUIRED",
];

pub(super) fn reported_cause(
	meddra_code: Option<&str>,
	meddra_version: Option<&str>,
	comments: Option<&str>,
) -> ReportedCauseOfDeath {
	ReportedCauseOfDeath {
		id: Uuid::nil(),
		death_info_id: Uuid::nil(),
		sequence_number: 0,
		meddra_version: meddra_version.map(str::to_string),
		meddra_code: meddra_code.map(str::to_string),
		comments: comments.map(str::to_string),
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn reported_cause_codes(cause: ReportedCauseOfDeath) -> Vec<(String, String)> {
	let mut ctx = empty_ctx();
	ctx.reported_causes_of_death = vec![cause];
	let mut issues = Vec::new();
	collect_ich_issues(&ctx, &mut issues);
	let mut out: Vec<(String, String)> = issues
		.into_iter()
		.filter(|issue| REPORTED_CAUSE_CODES.contains(&issue.code.as_str()))
		.map(|issue| (issue.code, issue.path))
		.collect();
	out.sort();
	out
}

pub(super) fn autopsy_cause(
	meddra_code: Option<&str>,
	meddra_version: Option<&str>,
	comments: Option<&str>,
) -> AutopsyCauseOfDeath {
	AutopsyCauseOfDeath {
		id: Uuid::nil(),
		death_info_id: Uuid::nil(),
		sequence_number: 0,
		meddra_version: meddra_version.map(str::to_string),
		meddra_code: meddra_code.map(str::to_string),
		comments: comments.map(str::to_string),
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn length_issue(code: &str, path: &str) -> (String, String) {
	(code.to_string(), path.to_string())
}

fn length_issues(ctx: &ValidationContext) -> Vec<(String, String)> {
	let mut issues = Vec::new();
	collect_ich_issues(ctx, &mut issues);
	let mut out = issues
		.into_iter()
		.filter(|issue| issue.code.contains(".LENGTH.MAX"))
		.map(|issue| (issue.code, issue.path))
		.collect::<Vec<_>>();
	out.sort();
	out
}

#[test]
fn reported_cause_present_without_comment_flags_or_trigger_rule() {
	// code + version present, comment missing -> only the OR-trigger D.9.2.r.2.
	assert_eq!(
		reported_cause_codes(reported_cause(Some("10000001"), Some("27.0"), None)),
		vec![(
			"ICH.D.9.2.r.2.REQUIRED".to_string(),
			"patientInformation.death.reportedCauses.0.comments".to_string()
		)]
	);
}

#[test]
fn reported_cause_fully_populated_is_silent() {
	assert_eq!(
		reported_cause_codes(reported_cause(
			Some("10000001"),
			Some("27.0"),
			Some("fatal")
		)),
		Vec::new()
	);
}

#[test]
fn max_length_rules_cover_d_patient_text_fields() {
	let mut patient = patient();
	patient.patient_initials = Some("P".repeat(61));
	patient.age_unit = Some("A".repeat(51));
	patient.gestation_period_unit = Some("G".repeat(51));
	patient.age_group = Some("AG".to_string());
	patient.sex = Some("SX".to_string());
	patient.medical_history_text = Some("H".repeat(10001));
	let mut ctx = empty_ctx();
	ctx.patient = Some(patient);

	assert_eq!(
		length_issues(&ctx),
		vec![
			length_issue("ICH.D.1.LENGTH.MAX", "patientInformation.patientInitials"),
			length_issue(
				"ICH.D.2.2.1b.LENGTH.MAX",
				"patientInformation.gestationPeriodUnit"
			),
			length_issue("ICH.D.2.2b.LENGTH.MAX", "patientInformation.ageUnit"),
			length_issue(
				"ICH.D.2.3.LENGTH.MAX",
				"patientInformation.patientAgeGroup"
			),
			length_issue("ICH.D.5.LENGTH.MAX", "patientInformation.sex"),
			length_issue(
				"ICH.D.7.2.LENGTH.MAX",
				"patientInformation.medicalHistoryText"
			),
		]
	);
}

#[test]
fn max_length_rules_cover_d_patient_child_text_fields() {
	let mut medical_history = medhist(Some(&"C".repeat(9)), Some(&"V".repeat(5)));
	medical_history.comments = Some("M".repeat(2001));
	let mut past_drug = past_drug();
	past_drug.drug_name = Some("D".repeat(251));
	past_drug.mpid_version = Some("V".repeat(11));
	past_drug.mpid = Some("M".repeat(1001));
	past_drug.phpid_version = Some("P".repeat(11));
	past_drug.phpid = Some("H".repeat(251));
	past_drug.indication_meddra_version = Some("I".repeat(5));
	past_drug.indication_meddra_code = Some("I".repeat(9));
	past_drug.reaction_meddra_version = Some("R".repeat(5));
	past_drug.reaction_meddra_code = Some("R".repeat(9));
	let reported = reported_cause(
		Some(&"C".repeat(9)),
		Some(&"V".repeat(5)),
		Some(&"R".repeat(251)),
	);
	let autopsy = autopsy_cause(
		Some(&"C".repeat(9)),
		Some(&"V".repeat(5)),
		Some(&"A".repeat(251)),
	);
	let mut ctx = empty_ctx();
	ctx.medical_history = vec![medical_history];
	ctx.past_drugs = vec![past_drug];
	ctx.reported_causes_of_death = vec![reported];
	ctx.autopsy_causes_of_death = vec![autopsy];

	assert_eq!(
		length_issues(&ctx),
		vec![
			length_issue(
				"ICH.D.7.1.r.1a.LENGTH.MAX",
				"patientInformation.medicalHistory.0.meddraVersion"
			),
			length_issue(
				"ICH.D.7.1.r.1b.LENGTH.MAX",
				"patientInformation.medicalHistory.0.meddraCode"
			),
			length_issue(
				"ICH.D.7.1.r.5.LENGTH.MAX",
				"patientInformation.medicalHistory.0.comments"
			),
			length_issue(
				"ICH.D.8.r.1.LENGTH.MAX",
				"patientInformation.pastDrugs.0.drugName"
			),
			length_issue(
				"ICH.D.8.r.2a.LENGTH.MAX",
				"patientInformation.pastDrugs.0.mpidVersion"
			),
			length_issue(
				"ICH.D.8.r.2b.LENGTH.MAX",
				"patientInformation.pastDrugs.0.mpid"
			),
			length_issue(
				"ICH.D.8.r.3a.LENGTH.MAX",
				"patientInformation.pastDrugs.0.phpidVersion"
			),
			length_issue(
				"ICH.D.8.r.3b.LENGTH.MAX",
				"patientInformation.pastDrugs.0.phpid"
			),
			length_issue(
				"ICH.D.8.r.6a.LENGTH.MAX",
				"patientInformation.pastDrugs.0.indicationMeddraVersion"
			),
			length_issue(
				"ICH.D.8.r.6b.LENGTH.MAX",
				"patientInformation.pastDrugs.0.indicationMeddraCode"
			),
			length_issue(
				"ICH.D.8.r.7a.LENGTH.MAX",
				"patientInformation.pastDrugs.0.reactionMeddraVersion"
			),
			length_issue(
				"ICH.D.8.r.7b.LENGTH.MAX",
				"patientInformation.pastDrugs.0.reactionMeddraCode"
			),
			length_issue(
				"ICH.D.9.2.r.1a.LENGTH.MAX",
				"patientInformation.death.reportedCauses.0.meddraVersion"
			),
			length_issue(
				"ICH.D.9.2.r.1b.LENGTH.MAX",
				"patientInformation.death.reportedCauses.0.meddraCode"
			),
			length_issue(
				"ICH.D.9.2.r.2.LENGTH.MAX",
				"patientInformation.death.reportedCauses.0.comments"
			),
			length_issue(
				"ICH.D.9.4.r.1a.LENGTH.MAX",
				"patientInformation.death.autopsyCauses.0.meddraVersion"
			),
			length_issue(
				"ICH.D.9.4.r.1b.LENGTH.MAX",
				"patientInformation.death.autopsyCauses.0.meddraCode"
			),
			length_issue(
				"ICH.D.9.4.r.2.LENGTH.MAX",
				"patientInformation.death.autopsyCauses.0.comments"
			),
		]
	);
}

#[test]
fn max_length_rules_cover_d_parent_text_fields() {
	let parent_id = Uuid::from_u128(1);
	let mut parent = parent(parent_id);
	parent.parent_identification = Some("P".repeat(61));
	parent.parent_age_unit = Some("A".repeat(51));
	parent.sex = Some("SX".to_string());
	parent.medical_history_text = Some("H".repeat(10001));
	let mut parent_medical_history =
		parent_medhist(parent_id, Some(&"C".repeat(9)), Some(&"V".repeat(5)));
	parent_medical_history.comments = Some("M".repeat(2001));
	let mut parent_past_drug = parent_past_drug(parent_id, None, None);
	parent_past_drug.drug_name = Some("D".repeat(251));
	parent_past_drug.mpid_version = Some("V".repeat(11));
	parent_past_drug.mpid = Some("M".repeat(1001));
	parent_past_drug.phpid_version = Some("P".repeat(11));
	parent_past_drug.phpid = Some("H".repeat(251));
	parent_past_drug.indication_meddra_version = Some("I".repeat(5));
	parent_past_drug.indication_meddra_code = Some("I".repeat(9));
	parent_past_drug.reaction_meddra_version = Some("R".repeat(5));
	parent_past_drug.reaction_meddra_code = Some("R".repeat(9));
	let mut ctx = empty_ctx();
	ctx.parents = vec![parent];
	ctx.parent_medical_history = vec![parent_medical_history];
	ctx.parent_past_drugs = vec![parent_past_drug];

	assert_eq!(
		length_issues(&ctx),
		vec![
			length_issue(
				"ICH.D.10.1.LENGTH.MAX",
				"patientInformation.parents.0.parentIdentification"
			),
			length_issue(
				"ICH.D.10.2.2b.LENGTH.MAX",
				"patientInformation.parents.0.parentAgeUnit"
			),
			length_issue(
				"ICH.D.10.6.LENGTH.MAX",
				"patientInformation.parents.0.sex"
			),
			length_issue(
				"ICH.D.10.7.1.r.1a.LENGTH.MAX",
				"patientInformation.parents.0.medicalHistory.0.meddraVersion"
			),
			length_issue(
				"ICH.D.10.7.1.r.1b.LENGTH.MAX",
				"patientInformation.parents.0.medicalHistory.0.meddraCode"
			),
			length_issue(
				"ICH.D.10.7.1.r.5.LENGTH.MAX",
				"patientInformation.parents.0.medicalHistory.0.comments"
			),
			length_issue(
				"ICH.D.10.7.2.LENGTH.MAX",
				"patientInformation.parents.0.medicalHistoryText"
			),
			length_issue(
				"ICH.D.10.8.r.1.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.drugName"
			),
			length_issue(
				"ICH.D.10.8.r.2a.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.mpidVersion"
			),
			length_issue(
				"ICH.D.10.8.r.2b.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.mpid"
			),
			length_issue(
				"ICH.D.10.8.r.3a.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.phpidVersion"
			),
			length_issue(
				"ICH.D.10.8.r.3b.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.phpid"
			),
			length_issue(
				"ICH.D.10.8.r.6a.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.indicationMeddraVersion"
			),
			length_issue(
				"ICH.D.10.8.r.6b.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.indicationMeddraCode"
			),
			length_issue(
				"ICH.D.10.8.r.7a.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.reactionMeddraVersion"
			),
			length_issue(
				"ICH.D.10.8.r.7b.LENGTH.MAX",
				"patientInformation.parents.0.pastDrugs.0.reactionMeddraCode"
			),
		]
	);
}

#[test]
fn max_length_rules_cover_d_identifier_and_decimal_fields() {
	let parent_id = Uuid::from_u128(1);
	let mut patient = patient();
	patient.age_at_time_of_onset = Some(Decimal::new(123456, 0));
	patient.gestation_period = Some(Decimal::new(1234, 0));
	patient.weight_kg = Some(Decimal::new(1234567, 0));
	patient.height_cm = Some(Decimal::new(1234, 0));
	let mut parent = parent(parent_id);
	parent.parent_age = Some(Decimal::new(1234, 0));
	parent.weight_kg = Some(Decimal::new(1234567, 0));
	parent.height_cm = Some(Decimal::new(1234, 0));
	let mut ctx = empty_ctx();
	ctx.patient = Some(patient);
	ctx.parents = vec![parent];
	ctx.patient_identifiers = vec![
		patient_identifier("1", "G".repeat(21).as_str()),
		patient_identifier("2", "S".repeat(21).as_str()),
		patient_identifier("3", "H".repeat(21).as_str()),
		patient_identifier("4", "I".repeat(21).as_str()),
	];

	assert_eq!(
		length_issues(&ctx),
		vec![
			length_issue(
				"ICH.D.1.1.1.LENGTH.MAX",
				"patientInformation.gpMedicalRecordNumber"
			),
			length_issue(
				"ICH.D.1.1.2.LENGTH.MAX",
				"patientInformation.specialistRecordNumber"
			),
			length_issue(
				"ICH.D.1.1.3.LENGTH.MAX",
				"patientInformation.hospitalRecordNumber"
			),
			length_issue(
				"ICH.D.1.1.4.LENGTH.MAX",
				"patientInformation.patientStudyNumber"
			),
			length_issue(
				"ICH.D.10.2.2a.LENGTH.MAX",
				"patientInformation.parents.0.parentAge"
			),
			length_issue(
				"ICH.D.10.4.LENGTH.MAX",
				"patientInformation.parents.0.weightKg"
			),
			length_issue(
				"ICH.D.10.5.LENGTH.MAX",
				"patientInformation.parents.0.heightCm"
			),
			length_issue(
				"ICH.D.2.2.1a.LENGTH.MAX",
				"patientInformation.gestationPeriod"
			),
			length_issue(
				"ICH.D.2.2a.LENGTH.MAX",
				"patientInformation.ageAtTimeOfOnset"
			),
			length_issue("ICH.D.3.LENGTH.MAX", "patientInformation.weightKg"),
			length_issue("ICH.D.4.LENGTH.MAX", "patientInformation.heightCm"),
		]
	);
}

#[test]
fn future_date_rules_cover_remaining_d_date_time_fields() {
	const FUTURE_CODES: &[&str] = &[
		"ICH.D.6.FUTURE_DATE.FORBIDDEN",
		"ICH.D.8.r.FUTURE_DATE.FORBIDDEN",
		"ICH.D.9.1.FUTURE_DATE.FORBIDDEN",
		"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN",
		"ICH.D.10.3.FUTURE_DATE.FORBIDDEN",
		"ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN",
		"ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN",
	];

	let future_date =
		Date::from_calendar_date(2999, Month::January, 1).expect("valid test date");
	let parent_id = Uuid::from_u128(1);
	let mut ctx = empty_ctx();
	let mut patient = patient();
	patient.last_menstrual_period_date = Some(future_date);
	ctx.patient = Some(patient);
	let mut past_drug = past_drug();
	past_drug.start_date = Some(future_date);
	ctx.past_drugs = vec![past_drug];
	let mut death_info = death_info();
	death_info.date_of_death = Some(future_date);
	ctx.death_info = Some(death_info);
	let mut parent = parent(parent_id);
	parent.parent_birth_date = Some(future_date);
	parent.last_menstrual_period_date = Some(future_date);
	parent.sex = Some("1".to_string());
	ctx.parents = vec![parent];
	let mut parent_medhist = parent_medhist(parent_id, None, None);
	parent_medhist.start_date = Some(future_date);
	ctx.parent_medical_history = vec![parent_medhist];
	let mut parent_past_drug = parent_past_drug(parent_id, None, None);
	parent_past_drug.end_date = Some(future_date);
	ctx.parent_past_drugs = vec![parent_past_drug];

	let mut issues = Vec::new();
	collect_ich_issues(&ctx, &mut issues);
	let mut out = issues
		.into_iter()
		.filter(|issue| FUTURE_CODES.contains(&issue.code.as_str()))
		.map(|issue| (issue.code, issue.path))
		.collect::<Vec<_>>();
	out.sort();

	assert_eq!(
		out,
		vec![
			(
				"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN".to_string(),
				"patientInformation.parents.0.parentBirthDate".to_string()
			),
			(
				"ICH.D.10.3.FUTURE_DATE.FORBIDDEN".to_string(),
				"patientInformation.parents.0.lastMenstrualPeriodDate".to_string()
			),
			(
				"ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN".to_string(),
				"patientInformation.parents.0.medicalHistory.0.dateRange"
					.to_string()
			),
			(
				"ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN".to_string(),
				"patientInformation.parents.0.pastDrugs.0.dateRange".to_string()
			),
			(
				"ICH.D.6.FUTURE_DATE.FORBIDDEN".to_string(),
				"patientInformation.lastMenstrualPeriodDate".to_string()
			),
			(
				"ICH.D.8.r.FUTURE_DATE.FORBIDDEN".to_string(),
				"patientInformation.pastDrugs.0.dateRange".to_string()
			),
			(
				"ICH.D.9.1.FUTURE_DATE.FORBIDDEN".to_string(),
				"patientInformation.death.dateOfDeath".to_string()
			),
		]
	);
}

#[test]
fn allowed_value_rules_cover_patient_and_parent_codes() {
	let mut ctx = empty_ctx();
	let mut patient = patient();
	patient.age_group = Some("9".to_string());
	patient.sex = Some("3".to_string());
	patient.concomitant_therapy = Some(false);
	ctx.patient = Some(patient);
	let mut episode = medhist(None, None);
	episode.family_history = Some(false);
	ctx.medical_history.push(episode);

	let mut parent = parent(Uuid::from_u128(1));
	parent.sex = Some("3".to_string());
	ctx.parents.push(parent);

	let mut issues = Vec::new();
	collect_ich_issues(&ctx, &mut issues);
	let mut out = issues
		.into_iter()
		.filter(|issue| issue.code.ends_with(".ALLOWED.VALUE"))
		.map(|issue| (issue.code, issue.field_path.unwrap_or_default()))
		.collect::<Vec<_>>();
	out.sort();

	assert_eq!(
		out,
		vec![
			(
				"ICH.D.10.6.ALLOWED.VALUE".to_string(),
				"patientInformation.parents.0.sex".to_string()
			),
			(
				"ICH.D.2.3.ALLOWED.VALUE".to_string(),
				"patientInformation.patientAgeGroup".to_string()
			),
			(
				"ICH.D.5.ALLOWED.VALUE".to_string(),
				"patientInformation.sex".to_string()
			),
			(
				"ICH.D.7.1.r.6.ALLOWED.VALUE".to_string(),
				"patientInformation.medicalHistory.0.familyHistory".to_string()
			),
			(
				"ICH.D.7.3.ALLOWED.VALUE".to_string(),
				"patientInformation.concomitantTherapy".to_string()
			),
		]
	);
}

#[test]
fn d_6_null_flavor_is_a_case_validation_rule() {
	let mut patient = patient();
	patient.last_menstrual_period_date_null_flavor = Some("NASK".to_string());
	let mut issues = Vec::new();
	d_6(&patient, &mut issues);
	assert_eq!(
		issues
			.iter()
			.map(|issue| issue.code.as_str())
			.collect::<Vec<_>>(),
		vec!["ICH.D.6.NULLFLAVOR.ALLOWED"]
	);
}

#[test]
fn fda_patient_rules_keep_catalog_conditions_and_paths() {
	let patient = patient();
	let mut issues = Vec::new();
	fda_d_11(Some(&patient), false, &mut issues);
	fda_d_12(Some(&patient), false, &mut issues);
	assert_eq!(
		issues
			.iter()
			.map(|issue| (issue.code.as_str(), issue.field_path.as_deref()))
			.collect::<Vec<_>>(),
		[
			(
				"FDA.D.11.r.1.REQUIRED",
				Some("patientInformation.raceCodes")
			),
			(
				"FDA.D.12.REQUIRED",
				Some("patientInformation.ethnicityCode")
			),
		]
	);
}

#[test]
fn age_descriptions_are_mutually_exclusive() {
	let mut patient = patient();
	patient.age_at_time_of_onset = Some(Decimal::ONE);
	patient.age_group = Some("3".to_string());
	let mut issues = Vec::new();
	d_2(&patient, &mut issues);
	assert!(issues.iter().any(|issue| issue.code == "ICH.D.2.EXCLUSIVE"));
}

#[test]
fn heights_must_be_whole_numbers() {
	let mut patient = patient();
	patient.height_cm = Some(Decimal::new(1755, 1));
	let mut issues = Vec::new();
	d_4_integer(&patient, &mut issues);
	assert!(issues.iter().any(|issue| issue.code == "ICH.D.4.INTEGER"));
}

#[test]
fn parent_height_integer_has_both_edges() {
	let mut parent = parent(Uuid::nil());
	parent.height_cm = Some(Decimal::new(1755, 1));
	let mut issues = Vec::new();
	d_10_5_integer(2, &parent, &mut issues);
	assert!(issues.iter().any(|issue| {
		issue.code == "ICH.D.10.5.INTEGER"
			&& issue.path == "patientInformation.parents.2.heightCm"
	}));

	issues.clear();
	parent.height_cm = Some(Decimal::new(175, 0));
	d_10_5_integer(2, &parent, &mut issues);
	assert!(issues
		.iter()
		.all(|issue| issue.code != "ICH.D.10.5.INTEGER"));
}

#[test]
fn parent_birth_date_and_age_are_exclusive() {
	let mut parent = parent(Uuid::nil());
	parent.parent_birth_date_null_flavor = Some("UNK".to_string());
	parent.parent_age = Some(Decimal::ONE);
	let mut issues = Vec::new();
	d_10_2(0, &parent, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "ICH.D.10.2.EXCLUSIVE"));
}

#[test]
fn fda_age_accepts_birth_date_null_flavor_and_skips_malfunction_only() {
	let mut patient = patient();
	patient.birth_date_null_flavor = Some("UNK".to_string());
	let mut issues = Vec::new();
	fda_d_2(Some(&patient), &mut issues);
	assert!(issues.is_empty());
	assert!(!fda_d_2_required(Some("5")));
}

#[test]
fn fda_race_and_ethnicity_accept_null_flavors() {
	let mut patient = patient();
	patient.race_code_null_flavor = Some("UNK".to_string());
	patient.ethnicity_code_null_flavor = Some("UNK".to_string());
	let mut issues = Vec::new();
	fda_d_11(Some(&patient), true, &mut issues);
	fda_d_12(Some(&patient), true, &mut issues);
	assert!(issues.is_empty());

	patient.race_code_null_flavor = Some("NA".to_string());
	patient.ethnicity_code_null_flavor = Some("NA".to_string());
	fda_d_11(Some(&patient), true, &mut issues);
	fda_d_12(Some(&patient), true, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.D.11.NULLFLAVOR.ROUTE"));
	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.D.12.NULLFLAVOR.ROUTE"));
}

#[test]
fn fda_race_accepts_repeated_official_codes_only() {
	let mut patient = patient();
	patient.race_codes = vec!["C16352".to_string(), "C41259".to_string()];
	let mut issues = Vec::new();
	fda_d_11(Some(&patient), false, &mut issues);
	assert!(issues.is_empty());

	patient.race_codes.push("INVALID".to_string());
	fda_d_11(Some(&patient), false, &mut issues);
	assert!(issues.iter().any(|issue| {
		issue.code == "FDA.D.11.r.1.ALLOWED.VALUE"
			&& issue.field_path.as_deref() == Some("patientInformation.raceCodes.2")
	}));
}

#[test]
fn fda_race_and_ethnicity_are_reported_when_patient_row_is_missing() {
	let mut issues = Vec::new();
	fda_d_11(None, true, &mut issues);
	fda_d_12(None, true, &mut issues);
	assert!(issues.iter().any(|issue| issue.code == "FDA.D.11.REQUIRED"));
	assert!(issues.iter().any(|issue| issue.code == "FDA.D.12.REQUIRED"));
}

#[test]
fn fda_death_date_rule_is_ind_study_only_and_accepts_null_flavor() {
	assert!(fda_d_9_1_required(Some("2"), Some("CDER_IND"), true, false));
	assert!(fda_d_9_1_required(
		Some("2"),
		Some("CDER_IND_EXEMPT_BA_BE"),
		true,
		false
	));
	assert!(!fda_d_9_1_required(
		Some("1"),
		Some("CDER_IND"),
		true,
		false
	));
	assert!(!fda_d_9_1_required(Some("2"), Some("CDER_IND"), true, true));
}

#[test]
fn parent_duplicate_uses_meddra_code_across_versions() {
	let parent_id = Uuid::new_v4();
	let mut episode = medhist(Some("10000001"), Some("26.0"));
	episode.family_history = Some(true);
	let parent_history = parent_medhist(parent_id, Some("10000001"), Some("27.0"));
	let mut issues = Vec::new();
	d_7_1_r_6_parent_duplicate(0, &episode, &[parent_history], &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "ICH.D.7.1.r.6.PARENT_DUPLICATE"));
}

#[test]
fn mfds_study_number_ignores_deleted_identifiers() {
	let mut ctx = empty_ctx();
	let mut identifier = patient_identifier("4", "STUDY-1");
	identifier.deleted = true;
	ctx.patient_identifiers = vec![identifier];
	let mut issues = Vec::new();
	mfds_d_ct_cu(&ctx, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "MFDS.D.1.1.4.REQUIRED"));
}

#[test]
fn mfds_age_group_excludes_foetus_without_narrowing_ich_or_fda() {
	let mfds = MfdsValidationContext {
		senders: Vec::new(),
		studies: Vec::new(),
		active_substances: Vec::new(),
		relatedness: Vec::new(),
		past_drugs: Vec::new(),
		parent_past_drugs: Vec::new(),
	};
	let mut ctx = empty_ctx();
	let mut value = patient();
	value.age_group = Some("0".to_string());
	ctx.patient = Some(value);

	for authority in [RegulatoryAuthority::Ich, RegulatoryAuthority::Fda] {
		let mut issues = Vec::new();
		collect(&mut issues, authority, &ctx, None, None);
		assert!(!issues.iter().any(|issue| {
			issue.code == "ICH.D.2.3.ALLOWED.VALUE"
				|| issue.code == "MFDS.D.2.3.ALLOWED.VALUE"
		}));
	}

	let mut issues = Vec::new();
	collect(
		&mut issues,
		RegulatoryAuthority::Mfds,
		&ctx,
		None,
		Some(&mfds),
	);
	let matches = issues
		.iter()
		.filter(|issue| issue.code == "MFDS.D.2.3.ALLOWED.VALUE")
		.collect::<Vec<_>>();
	assert_eq!(matches.len(), 1);
	assert_eq!(matches[0].path, "patientInformation.patientAgeGroup");
	assert_eq!(
		matches[0].field_path.as_deref(),
		Some("patientInformation.patientAgeGroup")
	);

	for code in ["1", "6"] {
		if let Some(patient) = ctx.patient.as_mut() {
			patient.age_group = Some(code.to_string());
		} else {
			panic!("test patient is required");
		}
		let mut issues = Vec::new();
		collect(
			&mut issues,
			RegulatoryAuthority::Mfds,
			&ctx,
			None,
			Some(&mfds),
		);
		assert!(!issues
			.iter()
			.any(|issue| issue.code == "MFDS.D.2.3.ALLOWED.VALUE"));
	}
}

#[test]
fn mfds_past_drug_rules_keep_concrete_indices() {
	let past = crate::PastDrugByCase {
		drug_name_null_flavor: None,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		mfds_medicinal_product_id: Some("product".to_string()),
		mfds_medicinal_product_version: None,
	};
	let ctx = empty_ctx();
	let mut issues = Vec::new();
	mfds_d_8_r_1_kr_1b(4, &past, true, Some("FR"), &ctx, &mut issues);
	mfds_d_8_r_1_kr_1a(4, &past, true, &mut issues);
	assert!(issues.iter().any(|issue| {
		issue.code == "MFDS.D.8.r.1.KR.1a.REQUIRED"
			&& issue.field_path.as_deref()
				== Some(
					"patientInformation.pastDrugHistory.4.mfdsMedicinalProductVersion",
				)
	}));
}

#[test]
fn mfds_past_drug_product_code_is_optional_with_name_null_flavor() {
	let past = crate::PastDrugByCase {
		drug_name_null_flavor: Some("UNK".to_string()),
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		mfds_medicinal_product_id: None,
		mfds_medicinal_product_version: None,
	};
	let mut issues = Vec::new();
	mfds_d_8_r_1_kr_1b(0, &past, true, Some("KR"), &empty_ctx(), &mut issues);
	assert!(!issues
		.iter()
		.any(|issue| issue.code == "MFDS.D.8.r.1.KR.1b.REQUIRED"));
}

#[test]
fn mfds_past_drug_identifiers_and_versions_are_paired() {
	let past = crate::PastDrugByCase {
		drug_name_null_flavor: None,
		mpid: Some("MPID".to_string()),
		mpid_version: None,
		phpid: None,
		phpid_version: Some("1".to_string()),
		mfds_medicinal_product_id: None,
		mfds_medicinal_product_version: None,
	};
	let mut issues = Vec::new();
	mfds_d_8_identifier_companions(0, &past, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "MFDS.D.8.r.2a.REQUIRED"));
	assert!(issues
		.iter()
		.any(|issue| issue.code == "MFDS.D.8.r.3b.REQUIRED"));
}

#[test]
fn golden_d_issue_metadata() {
	let mut issues = Vec::new();
	d_1(None, false, &mut issues);
	let mut patient = patient();
	patient.age_group = Some("9".to_string());
	d_2_3(&patient, &mut issues);
	let past = crate::PastDrugByCase {
		drug_name_null_flavor: None,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		mfds_medicinal_product_id: Some("product".to_string()),
		mfds_medicinal_product_version: None,
	};
	mfds_d_8_r_1_kr_1a(4, &past, true, &mut issues);

	let mut out = issues
		.into_iter()
		.filter(|issue| {
			matches!(
				issue.code.as_str(),
				"ICH.D.1.REQUIRED"
					| "ICH.D.2.3.ALLOWED.VALUE"
					| "MFDS.D.8.r.1.KR.1a.REQUIRED"
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
				"ICH.D.1.REQUIRED".to_string(),
				"[D.1] This Element is required.".to_string(),
				"patientInformation.patientInitials".to_string(),
				Some("patientInformation.patientInitials".to_string()),
				"patient".to_string(),
				"D.1".to_string(),
			),
			(
				"ICH.D.2.3.ALLOWED.VALUE".to_string(),
				"Dictionary allowed values constraint.".to_string(),
				"patientInformation.patientAgeGroup".to_string(),
				Some("patientInformation.patientAgeGroup".to_string()),
				"patient".to_string(),
				"D.2".to_string(),
			),
			(
				"MFDS.D.8.r.1.KR.1a.REQUIRED".to_string(),
				"MFDS requires past drug code version [D.8.r.1.KR.1a] for FR when [D.8.r.1.KR.1b] is provided.".to_string(),
				"patientInformation.pastDrugHistory.4.mfdsMedicinalProductVersion".to_string(),
				Some("patientInformation.pastDrugHistory.4.mfdsMedicinalProductVersion".to_string()),
				"patient".to_string(),
				"D.8.r".to_string(),
			),
		],
	);
}
