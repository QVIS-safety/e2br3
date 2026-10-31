use super::helpers::{
	e2b_ts_date, max_length, reject_future_date, reject_when, require, valid_code,
	valid_decimal, valid_dotted_version, valid_identifier, valid_meddra_term,
	valid_meddra_version, valid_mfds_product, DateValues,
};
use crate::context::VocabularyContext;
use crate::{
	has_patient_initials, has_text, is_mfds_domestic_receiver,
	is_mfds_foreign_postmarket_receiver, push_business_issue,
	should_require_patient_initials, FdaValidationContext, MfdsValidationContext,
	RegulatoryAuthority, ValidationContext, ValidationIssue,
};
use lib_core::model::parent_history::{ParentMedicalHistory, ParentPastDrugHistory};
use lib_core::model::patient::{
	AutopsyCauseOfDeath, MedicalHistoryEpisode, ParentInformation, PastDrugHistory,
	PatientDeathInformation, PatientIdentifier, PatientInformation,
	ReportedCauseOfDeath,
};
use lib_core::regulatory::{
	is_fda_ind_message_receiver, is_fda_premarket_message_receiver,
	is_mfds_clinical_trial_receiver, is_mfds_compassionate_use_receiver,
};
use sqlx::types::{Decimal, Uuid};
use std::collections::HashMap;

const SECTION: &str = "patient";
const MAX_LENGTH_MESSAGE: &str = "Dictionary max length exceeded.";
const ALLOWED_VALUE_MESSAGE: &str = "Dictionary allowed values constraint.";
const VOCABULARY_MESSAGE: &str = "Dictionary vocabulary constraint.";

fn decimal_text(value: Option<Decimal>) -> Option<String> {
	value.map(|value| value.to_string())
}

fn is_integer(value: Decimal) -> bool {
	value.fract() == Decimal::ZERO
}

/// ICH.D.2: only one patient age description may be used.
fn d_2(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	let groups = [
		patient.birth_date.is_some()
			|| has_text(patient.birth_date_null_flavor.as_deref()),
		patient.age_at_time_of_onset.is_some()
			|| has_text(patient.age_unit.as_deref()),
		patient.gestation_period.is_some()
			|| has_text(patient.gestation_period_unit.as_deref()),
		has_text(patient.age_group.as_deref()),
	];
	if groups.into_iter().filter(|present| *present).count() > 1 {
		push_business_issue(
			issues,
			"ICH.D.2.EXCLUSIVE",
			"patientInformation.patientBirthDate",
			"Only one patient age description may be provided",
		);
	}
}

fn past_drug_has_payload(past_drug: &PastDrugHistory) -> bool {
	has_text(past_drug.drug_name.as_deref())
		|| has_text(past_drug.drug_name_null_flavor.as_deref())
		|| has_text(past_drug.mpid.as_deref())
		|| has_text(past_drug.mpid_version.as_deref())
		|| has_text(past_drug.phpid.as_deref())
		|| has_text(past_drug.phpid_version.as_deref())
		|| past_drug.start_date.is_some()
		|| has_text(past_drug.start_date_null_flavor.as_deref())
		|| past_drug.end_date.is_some()
		|| has_text(past_drug.end_date_null_flavor.as_deref())
		|| has_text(past_drug.indication_meddra_version.as_deref())
		|| has_text(past_drug.indication_meddra_code.as_deref())
		|| has_text(past_drug.reaction_meddra_version.as_deref())
		|| has_text(past_drug.reaction_meddra_code.as_deref())
}

fn parent_index_by_id(parents: &[ParentInformation]) -> HashMap<Uuid, usize> {
	parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent.id, idx))
		.collect()
}

fn required_field(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	message: &str,
	present: bool,
) {
	require(issues, code, path, SECTION, message, present);
}

fn required_when(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	message: &str,
	trigger: bool,
	present: bool,
) {
	reject_when(issues, code, path, SECTION, message, trigger && !present);
}

fn length(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	value: Option<&str>,
	max: usize,
) {
	max_length(issues, code, path, SECTION, MAX_LENGTH_MESSAGE, value, max);
}

fn allowed(issues: &mut Vec<ValidationIssue>, code: &str, path: &str, valid: bool) {
	reject_when(issues, code, path, SECTION, ALLOWED_VALUE_MESSAGE, !valid);
}

fn vocabulary(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	valid: bool,
) {
	reject_when(issues, code, path, SECTION, VOCABULARY_MESSAGE, !valid);
}

#[allow(clippy::too_many_arguments)]
fn meddra(
	issues: &mut Vec<ValidationIssue>,
	vocabulary_ctx: &VocabularyContext,
	version_allowed_code: &str,
	code_allowed_code: &str,
	version_code: &str,
	code_code: &str,
	version_path: String,
	code_path: String,
	version: Option<&str>,
	code: Option<&str>,
) {
	allowed(
		issues,
		version_allowed_code,
		&version_path,
		valid_dotted_version(version),
	);
	allowed(issues, code_allowed_code, &code_path, valid_decimal(code));
	vocabulary(
		issues,
		version_code,
		&version_path,
		valid_meddra_version(vocabulary_ctx, version),
	);
	vocabulary(
		issues,
		code_code,
		&code_path,
		valid_meddra_term(vocabulary_ctx, version, code),
	);
}

/// ICH.D.1.REQUIRED
/// ICH.D.1.LENGTH.MAX
fn d_1(patient: Option<&PatientInformation>, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "patientInformation.patientInitials";
	let present = patient.is_some_and(|patient| {
		!should_require_patient_initials(patient) || has_patient_initials(patient)
	});
	required_field(
		issues,
		"ICH.D.1.REQUIRED",
		PATH,
		"[D.1] This Element is required.",
		present,
	);
	length(
		issues,
		"ICH.D.1.LENGTH.MAX",
		PATH,
		patient.and_then(|patient| patient.patient_initials.as_deref()),
		60,
	);
}

fn patient_identifier_value<'a>(
	identifier: &'a PatientIdentifier,
	type_code: &str,
) -> Option<&'a str> {
	(identifier.identifier_type_code.trim() == type_code)
		.then_some(identifier.identifier_value.as_deref())
		.flatten()
}

macro_rules! patient_identifier_length {
	($name:ident, $code:literal, $type_code:literal, $path:literal) => {
		#[doc = $code]
		fn $name(identifier: &PatientIdentifier, issues: &mut Vec<ValidationIssue>) {
			length(
				issues,
				$code,
				$path,
				patient_identifier_value(identifier, $type_code),
				20,
			);
		}
	};
}

patient_identifier_length!(
	d_1_1_1,
	"ICH.D.1.1.1.LENGTH.MAX",
	"1",
	"patientInformation.gpMedicalRecordNumber"
);
patient_identifier_length!(
	d_1_1_2,
	"ICH.D.1.1.2.LENGTH.MAX",
	"2",
	"patientInformation.specialistRecordNumber"
);
patient_identifier_length!(
	d_1_1_3,
	"ICH.D.1.1.3.LENGTH.MAX",
	"3",
	"patientInformation.hospitalRecordNumber"
);

/// ICH.D.1.1.4.LENGTH.MAX
fn d_1_1_4(identifiers: &[PatientIdentifier], issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "patientInformation.patientStudyNumber";
	for identifier in identifiers {
		length(
			issues,
			"ICH.D.1.1.4.LENGTH.MAX",
			PATH,
			patient_identifier_value(identifier, "4"),
			20,
		);
	}
}

/// ICH.D.2.1.FUTURE_DATE.FORBIDDEN
fn d_2_1(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	reject_future_date(
		issues,
		"ICH.D.2.1.FUTURE_DATE.FORBIDDEN",
		"patientInformation.patientBirthDate",
		SECTION,
		"[D.2.1] Date of birth must not be later than today.",
		DateValues::One(patient.birth_date),
	);
}

/// ICH.D.2.2a.REQUIRED
/// ICH.D.2.2a.LENGTH.MAX
fn d_2_2a(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "patientInformation.ageAtTimeOfOnset";
	let value = decimal_text(patient.age_at_time_of_onset);
	required_when(
		issues,
		"ICH.D.2.2a.REQUIRED",
		PATH,
		"[D.2.2a] Age at time of onset is required when [D.2.2b] is provided.",
		has_text(patient.age_unit.as_deref()),
		value.is_some(),
	);
	length(issues, "ICH.D.2.2a.LENGTH.MAX", PATH, value.as_deref(), 5);
}

/// ICH.D.2.2b.REQUIRED
/// ICH.D.2.2b.LENGTH.MAX
fn d_2_2b(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "patientInformation.ageUnit";
	required_when(
		issues,
		"ICH.D.2.2b.REQUIRED",
		PATH,
		"[D.2.2b] Age unit is required when [D.2.2a] is provided.",
		patient.age_at_time_of_onset.is_some(),
		has_text(patient.age_unit.as_deref()),
	);
	length(
		issues,
		"ICH.D.2.2b.LENGTH.MAX",
		PATH,
		patient.age_unit.as_deref(),
		50,
	);
}

/// ICH.D.2.2.1a.REQUIRED
/// ICH.D.2.2.1a.LENGTH.MAX
fn d_2_2_1a(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "patientInformation.gestationPeriod";
	let value = decimal_text(patient.gestation_period);
	required_when(
		issues,
		"ICH.D.2.2.1a.REQUIRED",
		PATH,
		"[D.2.2.1a] Gestation period is required when [D.2.2.1b] is provided.",
		has_text(patient.gestation_period_unit.as_deref()),
		value.is_some(),
	);
	length(issues, "ICH.D.2.2.1a.LENGTH.MAX", PATH, value.as_deref(), 3);
}

/// ICH.D.2.2.1b.REQUIRED
/// ICH.D.2.2.1b.LENGTH.MAX
fn d_2_2_1b(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "patientInformation.gestationPeriodUnit";
	required_when(
		issues,
		"ICH.D.2.2.1b.REQUIRED",
		PATH,
		"[D.2.2.1b] Gestation period unit is required when [D.2.2.1a] is provided.",
		patient.gestation_period.is_some(),
		has_text(patient.gestation_period_unit.as_deref()),
	);
	length(
		issues,
		"ICH.D.2.2.1b.LENGTH.MAX",
		PATH,
		patient.gestation_period_unit.as_deref(),
		50,
	);
}

/// ICH.D.2.3.ALLOWED.VALUE
/// ICH.D.2.3.LENGTH.MAX
fn d_2_3(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "patientInformation.patientAgeGroup";
	allowed(
		issues,
		"ICH.D.2.3.ALLOWED.VALUE",
		PATH,
		valid_code(
			patient.age_group.as_deref(),
			&["0", "1", "2", "3", "4", "5", "6"],
		),
	);
	length(
		issues,
		"ICH.D.2.3.LENGTH.MAX",
		PATH,
		patient.age_group.as_deref(),
		1,
	);
}

/// MFDS.D.2.3.ALLOWED.VALUE
fn mfds_d_2_3(
	patient: Option<&PatientInformation>,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some(patient) = patient else {
		return;
	};
	allowed(
		issues,
		"MFDS.D.2.3.ALLOWED.VALUE",
		"patientInformation.patientAgeGroup",
		valid_code(
			patient.age_group.as_deref(),
			&["1", "2", "3", "4", "5", "6"],
		),
	);
}

macro_rules! patient_decimal_length {
	($name:ident, $code:literal, $path:literal, $field:ident, $max_length:literal) => {
		#[doc = $code]
		fn $name(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
			let value = decimal_text(patient.$field);
			length(issues, $code, $path, value.as_deref(), $max_length);
		}
	};
}

patient_decimal_length!(
	d_3,
	"ICH.D.3.LENGTH.MAX",
	"patientInformation.weightKg",
	weight_kg,
	6
);

/// ICH.D.4: height is a rounded integer.
fn d_4_integer(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	if patient.height_cm.is_some_and(|value| !is_integer(value)) {
		push_business_issue(
			issues,
			"ICH.D.4.INTEGER",
			"patientInformation.heightCm",
			"Patient height must be a whole number",
		);
	}
}
patient_decimal_length!(
	d_4,
	"ICH.D.4.LENGTH.MAX",
	"patientInformation.heightCm",
	height_cm,
	3
);

/// ICH.D.5.ALLOWED.VALUE
/// ICH.D.5.LENGTH.MAX
fn d_5(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "patientInformation.sex";
	allowed(
		issues,
		"ICH.D.5.ALLOWED.VALUE",
		PATH,
		valid_code(patient.sex.as_deref(), &["1", "2"]),
	);
	length(
		issues,
		"ICH.D.5.LENGTH.MAX",
		PATH,
		patient.sex.as_deref(),
		1,
	);
}

/// ICH.D.6.NULLFLAVOR.ALLOWED
/// ICH.D.6.FUTURE_DATE.FORBIDDEN
fn d_6(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	allowed(
		issues,
		"ICH.D.6.NULLFLAVOR.ALLOWED",
		"patientInformation.lastMenstrualPeriodDateNullFlavor",
		valid_code(
			patient.last_menstrual_period_date_null_flavor.as_deref(),
			&["MSK"],
		),
	);
	reject_future_date(
		issues,
		"ICH.D.6.FUTURE_DATE.FORBIDDEN",
		"patientInformation.lastMenstrualPeriodDate",
		SECTION,
		"[D.6] Last menstrual period date must not be later than today.",
		DateValues::One(patient.last_menstrual_period_date),
	);
}

/// ICH.D.7.2.REQUIRED
/// ICH.D.7.2.LENGTH.MAX
fn d_7_2(
	patient: &PatientInformation,
	medical_history_is_empty: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	const PATH: &str = "patientInformation.medicalHistoryText";
	if medical_history_is_empty {
		required_field(
			issues,
			"ICH.D.7.2.REQUIRED",
			PATH,
			"[D.7.2] is required.",
			has_text(patient.medical_history_text.as_deref())
				|| patient.medical_history_text_null_flavor.is_some(),
		);
	}
	length(
		issues,
		"ICH.D.7.2.LENGTH.MAX",
		PATH,
		patient.medical_history_text.as_deref(),
		10000,
	);
}

/// ICH.D.7.3.ALLOWED.VALUE
fn d_7_3(patient: &PatientInformation, issues: &mut Vec<ValidationIssue>) {
	allowed(
		issues,
		"ICH.D.7.3.ALLOWED.VALUE",
		"patientInformation.concomitantTherapy",
		patient.concomitant_therapy != Some(false),
	);
}

/// ICH.D.7.1.r.1a.REQUIRED
/// ICH.D.7.1.r.1a.LENGTH.MAX
fn d_7_1_r_1a(
	idx: usize,
	episode: &MedicalHistoryEpisode,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("patientInformation.medicalHistory.{idx}.meddraVersion");
	required_when(
		issues,
		"ICH.D.7.1.r.1a.REQUIRED",
		&path,
		"[D.7.1.r.1a] MedDRA version for medical history is required when [D.7.1.r.1b] is provided.",
		has_text(episode.meddra_code.as_deref()),
		has_text(episode.meddra_version.as_deref()),
	);
	length(
		issues,
		"ICH.D.7.1.r.1a.LENGTH.MAX",
		&path,
		episode.meddra_version.as_deref(),
		4,
	);
}

/// ICH.D.7.1.r.1b.REQUIRED
/// ICH.D.7.1.r.1b.LENGTH.MAX
fn d_7_1_r_1b(
	idx: usize,
	episode: &MedicalHistoryEpisode,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("patientInformation.medicalHistory.{idx}.meddraCode");
	required_when(
		issues,
		"ICH.D.7.1.r.1b.REQUIRED",
		&path,
		"[D.7.1.r.1b] Medical history MedDRA code is required when [D.7.1.r.1a] is provided.",
		has_text(episode.meddra_version.as_deref()),
		has_text(episode.meddra_code.as_deref()),
	);
	length(
		issues,
		"ICH.D.7.1.r.1b.LENGTH.MAX",
		&path,
		episode.meddra_code.as_deref(),
		8,
	);
}

/// ICH.D.7.1.r.1a.ALLOWED.VALUE
/// ICH.D.7.1.r.1a.VOCABULARY
/// ICH.D.7.1.r.1b.ALLOWED.VALUE
/// ICH.D.7.1.r.1b.VOCABULARY
fn d_7_1_r_1_meddra(
	idx: usize,
	episode: &MedicalHistoryEpisode,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	meddra(
		issues,
		&validation_ctx.vocabulary,
		"ICH.D.7.1.r.1a.ALLOWED.VALUE",
		"ICH.D.7.1.r.1b.ALLOWED.VALUE",
		"ICH.D.7.1.r.1a.VOCABULARY",
		"ICH.D.7.1.r.1b.VOCABULARY",
		format!("patientInformation.medicalHistory.{idx}.meddraVersion"),
		format!("patientInformation.medicalHistory.{idx}.meddraCode"),
		episode.meddra_version.as_deref(),
		episode.meddra_code.as_deref(),
	);
}

/// ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN
fn d_7_1_r(
	idx: usize,
	episode: &MedicalHistoryEpisode,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_future_date(
		issues,
		"ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN",
		&format!("patientInformation.medicalHistoryEpisodes.{idx}.dateRange"),
		SECTION,
		"[D.7.1.r] Medical history dates must not be later than today.",
		DateValues::Two(
			e2b_ts_date(episode.start_date.as_deref()),
			e2b_ts_date(episode.end_date.as_deref()),
		),
	);
}

/// ICH.D.7.1.r.5.LENGTH.MAX
fn d_7_1_r_5(
	idx: usize,
	episode: &MedicalHistoryEpisode,
	issues: &mut Vec<ValidationIssue>,
) {
	length(
		issues,
		"ICH.D.7.1.r.5.LENGTH.MAX",
		&format!("patientInformation.medicalHistory.{idx}.comments"),
		episode.comments.as_deref(),
		2000,
	);
}

/// ICH.D.7.1.r.6.ALLOWED.VALUE
fn d_7_1_r_6(
	idx: usize,
	episode: &MedicalHistoryEpisode,
	issues: &mut Vec<ValidationIssue>,
) {
	allowed(
		issues,
		"ICH.D.7.1.r.6.ALLOWED.VALUE",
		&format!("patientInformation.medicalHistory.{idx}.familyHistory"),
		episode.family_history != Some(false),
	);
}

/// ICH.D.7.1.r.6: a concept also reported for the parent is not family history.
fn d_7_1_r_6_parent_duplicate(
	idx: usize,
	episode: &MedicalHistoryEpisode,
	parent_history: &[ParentMedicalHistory],
	issues: &mut Vec<ValidationIssue>,
) {
	let duplicate = episode.family_history == Some(true)
		&& episode.meddra_code.as_deref().is_some_and(|code| {
			parent_history
				.iter()
				.any(|parent| parent.meddra_code.as_deref() == Some(code))
		});
	if duplicate {
		push_business_issue(
			issues,
			"ICH.D.7.1.r.6.PARENT_DUPLICATE",
			format!("patientInformation.medicalHistory.{idx}.familyHistory"),
			"Family history must be false when the same concept is reported for the parent",
		);
	}
}

/// ICH.D.8.r.1.REQUIRED
/// ICH.D.8.r.1.LENGTH.MAX
fn d_8_r_1(idx: usize, drug: &PastDrugHistory, issues: &mut Vec<ValidationIssue>) {
	let path = format!("patientInformation.pastDrugs.{idx}.drugName");
	if past_drug_has_payload(drug) {
		required_field(
			issues,
			"ICH.D.8.r.1.REQUIRED",
			&path,
			"[D.8.r.1] is required.",
			has_text(drug.drug_name.as_deref())
				|| drug.drug_name_null_flavor.is_some(),
		);
	}
	length(
		issues,
		"ICH.D.8.r.1.LENGTH.MAX",
		&path,
		drug.drug_name.as_deref(),
		250,
	);
}

/// ICH.D.8.r.2a.LENGTH.MAX
fn d_8_r_2a(idx: usize, drug: &PastDrugHistory, issues: &mut Vec<ValidationIssue>) {
	length(
		issues,
		"ICH.D.8.r.2a.LENGTH.MAX",
		&format!("patientInformation.pastDrugs.{idx}.mpidVersion"),
		drug.mpid_version.as_deref(),
		10,
	);
}

/// ICH.D.8.r.2b.ALLOWED.VALUE
/// ICH.D.8.r.2b.LENGTH.MAX
fn d_8_r_2b(idx: usize, drug: &PastDrugHistory, issues: &mut Vec<ValidationIssue>) {
	let path = format!("patientInformation.pastDrugs.{idx}.mpid");
	allowed(
		issues,
		"ICH.D.8.r.2b.ALLOWED.VALUE",
		&path,
		valid_identifier(drug.mpid.as_deref(), 1000),
	);
	length(
		issues,
		"ICH.D.8.r.2b.LENGTH.MAX",
		&path,
		drug.mpid.as_deref(),
		1000,
	);
}

/// ICH.D.8.r.3a.LENGTH.MAX
fn d_8_r_3a(idx: usize, drug: &PastDrugHistory, issues: &mut Vec<ValidationIssue>) {
	length(
		issues,
		"ICH.D.8.r.3a.LENGTH.MAX",
		&format!("patientInformation.pastDrugs.{idx}.phpidVersion"),
		drug.phpid_version.as_deref(),
		10,
	);
}

/// ICH.D.8.r.3b.ALLOWED.VALUE
/// ICH.D.8.r.3b.LENGTH.MAX
fn d_8_r_3b(idx: usize, drug: &PastDrugHistory, issues: &mut Vec<ValidationIssue>) {
	let path = format!("patientInformation.pastDrugs.{idx}.phpid");
	allowed(
		issues,
		"ICH.D.8.r.3b.ALLOWED.VALUE",
		&path,
		valid_identifier(drug.phpid.as_deref(), 250),
	);
	length(
		issues,
		"ICH.D.8.r.3b.LENGTH.MAX",
		&path,
		drug.phpid.as_deref(),
		250,
	);
}

macro_rules! past_drug_meddra_field {
	($name:ident, $suffix:literal, $label:literal, $path:literal, $version:ident, $code:ident) => {
		#[doc = concat!("ICH.D.8.r.", $suffix, "a.REQUIRED")]
		#[doc = concat!("ICH.D.8.r.", $suffix, "a.LENGTH.MAX")]
		#[doc = concat!("ICH.D.8.r.", $suffix, "b.REQUIRED")]
		#[doc = concat!("ICH.D.8.r.", $suffix, "b.LENGTH.MAX")]
		#[doc = concat!("ICH.D.8.r.", $suffix, "a.ALLOWED.VALUE")]
		#[doc = concat!("ICH.D.8.r.", $suffix, "a.VOCABULARY")]
		#[doc = concat!("ICH.D.8.r.", $suffix, "b.ALLOWED.VALUE")]
		#[doc = concat!("ICH.D.8.r.", $suffix, "b.VOCABULARY")]
		fn $name(
			idx: usize,
			drug: &PastDrugHistory,
			validation_ctx: &ValidationContext,
			issues: &mut Vec<ValidationIssue>,
		) {
			let version_path =
				format!("patientInformation.pastDrugs.{idx}.{}MeddraVersion", $path);
			let code_path =
				format!("patientInformation.pastDrugs.{idx}.{}MeddraCode", $path);
			required_when(
				issues,
				concat!("ICH.D.8.r.", $suffix, "a.REQUIRED"),
				&version_path,
				concat!(
					"[D.8.r.",
					$suffix,
					"a] ",
					$label,
					" MedDRA version is required when [D.8.r.",
					$suffix,
					"b] is provided."
				),
				has_text(drug.$code.as_deref()),
				has_text(drug.$version.as_deref()),
			);
			required_when(
				issues,
				concat!("ICH.D.8.r.", $suffix, "b.REQUIRED"),
				&code_path,
				concat!(
					"[D.8.r.",
					$suffix,
					"b] ",
					$label,
					" MedDRA code is required when [D.8.r.",
					$suffix,
					"a] is provided."
				),
				has_text(drug.$version.as_deref()),
				has_text(drug.$code.as_deref()),
			);
			length(
				issues,
				concat!("ICH.D.8.r.", $suffix, "a.LENGTH.MAX"),
				&version_path,
				drug.$version.as_deref(),
				4,
			);
			length(
				issues,
				concat!("ICH.D.8.r.", $suffix, "b.LENGTH.MAX"),
				&code_path,
				drug.$code.as_deref(),
				8,
			);
			meddra(
				issues,
				&validation_ctx.vocabulary,
				concat!("ICH.D.8.r.", $suffix, "a.ALLOWED.VALUE"),
				concat!("ICH.D.8.r.", $suffix, "b.ALLOWED.VALUE"),
				concat!("ICH.D.8.r.", $suffix, "a.VOCABULARY"),
				concat!("ICH.D.8.r.", $suffix, "b.VOCABULARY"),
				version_path,
				code_path,
				drug.$version.as_deref(),
				drug.$code.as_deref(),
			);
		}
	};
}

past_drug_meddra_field!(
	d_8_r_6,
	"6",
	"Indication",
	"indication",
	indication_meddra_version,
	indication_meddra_code
);
past_drug_meddra_field!(
	d_8_r_7,
	"7",
	"Reaction",
	"reaction",
	reaction_meddra_version,
	reaction_meddra_code
);

/// ICH.D.8.r.FUTURE_DATE.FORBIDDEN
/// ICH.D.8.MPID_PHPID.EXCLUSIVE
fn d_8_r(idx: usize, drug: &PastDrugHistory, issues: &mut Vec<ValidationIssue>) {
	reject_future_date(
		issues,
		"ICH.D.8.r.FUTURE_DATE.FORBIDDEN",
		&format!("patientInformation.pastDrugs.{idx}.dateRange"),
		SECTION,
		"[D.8.r.4/D.8.r.5] Past drug dates must not be later than today.",
		DateValues::Two(drug.start_date, drug.end_date),
	);
	reject_when(
		issues,
		"ICH.D.8.MPID_PHPID.EXCLUSIVE",
		&format!("patientInformation.pastDrugs.{idx}.mpid"),
		SECTION,
		"[D.8.r.2b/D.8.r.3b] Any given past drug entry may have either MPID or PhPID, but not both.",
		has_text(drug.mpid.as_deref()) && has_text(drug.phpid.as_deref()),
	);
}

/// ICH.D.9.1.FUTURE_DATE.FORBIDDEN
fn d_9_1(death: &PatientDeathInformation, issues: &mut Vec<ValidationIssue>) {
	reject_future_date(
		issues,
		"ICH.D.9.1.FUTURE_DATE.FORBIDDEN",
		"patientInformation.death.dateOfDeath",
		SECTION,
		"[D.9.1] Date of death must not be later than today.",
		DateValues::One(death.date_of_death),
	);
}

macro_rules! death_cause_functions {
	(
		$type:ty,
		$version_fn:ident,
		$code_fn:ident,
		$comments_fn:ident,
		$meddra_fn:ident,
		$prefix:literal,
		$display_prefix:literal,
		$subject:literal,
		$path:literal
	) => {
		#[doc = concat!($prefix, ".1a.REQUIRED")]
		#[doc = concat!($prefix, ".1a.LENGTH.MAX")]
		fn $version_fn(
			idx: usize,
			cause: &$type,
			issues: &mut Vec<ValidationIssue>,
		) {
			let path =
				format!("patientInformation.death.{}.{idx}.meddraVersion", $path);
			required_when(
				issues,
				concat!($prefix, ".1a.REQUIRED"),
				&path,
				concat!(
					"[",
					$display_prefix,
					".1a] ",
					$subject,
					" MedDRA version is required when [",
					$display_prefix,
					".1b] is provided."
				),
				has_text(cause.meddra_code.as_deref()),
				has_text(cause.meddra_version.as_deref()),
			);
			length(
				issues,
				concat!($prefix, ".1a.LENGTH.MAX"),
				&path,
				cause.meddra_version.as_deref(),
				4,
			);
		}

		#[doc = concat!($prefix, ".1b.REQUIRED")]
		#[doc = concat!($prefix, ".1b.LENGTH.MAX")]
		fn $code_fn(idx: usize, cause: &$type, issues: &mut Vec<ValidationIssue>) {
			let path =
				format!("patientInformation.death.{}.{idx}.meddraCode", $path);
			required_when(
				issues,
				concat!($prefix, ".1b.REQUIRED"),
				&path,
				concat!(
					"[",
					$display_prefix,
					".1b] ",
					$subject,
					" MedDRA code is required when [",
					$display_prefix,
					".1a] is provided."
				),
				has_text(cause.meddra_version.as_deref()),
				has_text(cause.meddra_code.as_deref()),
			);
			length(
				issues,
				concat!($prefix, ".1b.LENGTH.MAX"),
				&path,
				cause.meddra_code.as_deref(),
				8,
			);
		}

		#[doc = concat!($prefix, ".2.REQUIRED")]
		#[doc = concat!($prefix, ".2.LENGTH.MAX")]
		fn $comments_fn(
			idx: usize,
			cause: &$type,
			issues: &mut Vec<ValidationIssue>,
		) {
			let path = format!("patientInformation.death.{}.{idx}.comments", $path);
			required_when(
				issues,
				concat!($prefix, ".2.REQUIRED"),
				&path,
				concat!(
					"[",
					$display_prefix,
					".2] ",
					$subject,
					" comments are required when [",
					$display_prefix,
					".1] is populated."
				),
				has_text(cause.meddra_code.as_deref())
					|| has_text(cause.meddra_version.as_deref()),
				has_text(cause.comments.as_deref()),
			);
			length(
				issues,
				concat!($prefix, ".2.LENGTH.MAX"),
				&path,
				cause.comments.as_deref(),
				250,
			);
		}

		#[doc = concat!($prefix, ".1a.ALLOWED.VALUE")]
		#[doc = concat!($prefix, ".1a.VOCABULARY")]
		#[doc = concat!($prefix, ".1b.ALLOWED.VALUE")]
		#[doc = concat!($prefix, ".1b.VOCABULARY")]
		fn $meddra_fn(
			idx: usize,
			cause: &$type,
			validation_ctx: &ValidationContext,
			issues: &mut Vec<ValidationIssue>,
		) {
			meddra(
				issues,
				&validation_ctx.vocabulary,
				concat!($prefix, ".1a.ALLOWED.VALUE"),
				concat!($prefix, ".1b.ALLOWED.VALUE"),
				concat!($prefix, ".1a.VOCABULARY"),
				concat!($prefix, ".1b.VOCABULARY"),
				format!("patientInformation.death.{}.{idx}.meddraVersion", $path),
				format!("patientInformation.death.{}.{idx}.meddraCode", $path),
				cause.meddra_version.as_deref(),
				cause.meddra_code.as_deref(),
			);
		}
	};
}

death_cause_functions!(
	ReportedCauseOfDeath,
	d_9_2_r_1a,
	d_9_2_r_1b,
	d_9_2_r_2,
	d_9_2_r_1_meddra,
	"ICH.D.9.2.r",
	"D.9.2.r",
	"Reported cause of death",
	"reportedCauses"
);
death_cause_functions!(
	AutopsyCauseOfDeath,
	d_9_4_r_1a,
	d_9_4_r_1b,
	d_9_4_r_2,
	d_9_4_r_1_meddra,
	"ICH.D.9.4.r",
	"D.9.4.r",
	"Autopsy cause of death",
	"autopsyCauses"
);

/// ICH.D.9.3.REQUIRED
fn d_9_3(death: &PatientDeathInformation, issues: &mut Vec<ValidationIssue>) {
	required_when(
		issues,
		"ICH.D.9.3.REQUIRED",
		"patientInformation.death.autopsyPerformed",
		"[D.9.3] Autopsy was performed is required when [D.9.1] date of death is provided.",
		death.date_of_death.is_some()
			|| has_text(death.date_of_death_null_flavor.as_deref()),
		death.autopsy_performed.is_some()
			|| has_text(death.autopsy_performed_null_flavor.as_deref()),
	);
}

/// ICH.D.10.1.LENGTH.MAX
fn d_10_1(
	idx: usize,
	parent: &ParentInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	length(
		issues,
		"ICH.D.10.1.LENGTH.MAX",
		&format!("patientInformation.parents.{idx}.parentIdentification"),
		parent.parent_identification.as_deref(),
		60,
	);
}

/// ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN
fn d_10_2_1(
	idx: usize,
	parent: &ParentInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_future_date(
		issues,
		"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN",
		&format!("patientInformation.parents.{idx}.parentBirthDate"),
		SECTION,
		"[D.10.2.1] Parent date of birth must not be later than today.",
		DateValues::One(parent.parent_birth_date),
	);
}

/// ICH.D.10.2.2a.REQUIRED
/// ICH.D.10.2.2a.LENGTH.MAX
fn d_10_2_2a(
	idx: usize,
	parent: &ParentInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("patientInformation.parents.{idx}.parentAge");
	required_when(
		issues,
		"ICH.D.10.2.2a.REQUIRED",
		&path,
		"[D.10.2.2a] Parent age is required when [D.10.2.2b] is provided.",
		has_text(parent.parent_age_unit.as_deref()),
		parent.parent_age.is_some(),
	);
	let value = decimal_text(parent.parent_age);
	length(
		issues,
		"ICH.D.10.2.2a.LENGTH.MAX",
		&path,
		value.as_deref(),
		3,
	);
}

/// ICH.D.10.2.2b.REQUIRED
/// ICH.D.10.2.2b.ALLOWED.VALUE
/// ICH.D.10.2.2b.LENGTH.MAX
fn d_10_2_2b(
	idx: usize,
	parent: &ParentInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("patientInformation.parents.{idx}.parentAgeUnit");
	required_when(
		issues,
		"ICH.D.10.2.2b.REQUIRED",
		&path,
		"[D.10.2.2b] Parent age unit is required when [D.10.2.2a] is provided.",
		parent.parent_age.is_some(),
		has_text(parent.parent_age_unit.as_deref()),
	);
	reject_when(
		issues,
		"ICH.D.10.2.2b.ALLOWED.VALUE",
		&path,
		SECTION,
		"[D.10.2.2b] Parent age unit must be years (a) or decades (10.a).",
		parent
			.parent_age_unit
			.as_deref()
			.map(str::trim)
			.is_some_and(|value| !matches!(value, "a" | "10.a")),
	);
	length(
		issues,
		"ICH.D.10.2.2b.LENGTH.MAX",
		&path,
		parent.parent_age_unit.as_deref(),
		50,
	);
}

/// ICH.D.10.2: use either the parent's birth date or age, not both.
fn d_10_2(
	idx: usize,
	parent: &ParentInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let has_birth_date = parent.parent_birth_date.is_some()
		|| has_text(parent.parent_birth_date_null_flavor.as_deref());
	let has_age =
		parent.parent_age.is_some() || has_text(parent.parent_age_unit.as_deref());
	if has_birth_date && has_age {
		push_business_issue(
			issues,
			"ICH.D.10.2.EXCLUSIVE",
			format!("patientInformation.parents.{idx}.parentBirthDate"),
			"Use either the parent's birth date or age, not both",
		);
	}
}

/// ICH.D.10.3.FUTURE_DATE.FORBIDDEN
fn d_10_3(
	idx: usize,
	parent: &ParentInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_future_date(
		issues,
		"ICH.D.10.3.FUTURE_DATE.FORBIDDEN",
		&format!("patientInformation.parents.{idx}.lastMenstrualPeriodDate"),
		SECTION,
		"[D.10.3] Parent last menstrual period date must not be later than today.",
		DateValues::One(parent.last_menstrual_period_date),
	);
}

macro_rules! parent_decimal_length {
	($name:ident, $code:literal, $path:literal, $field:ident, $max_length:literal) => {
		#[doc = $code]
		fn $name(
			idx: usize,
			parent: &ParentInformation,
			issues: &mut Vec<ValidationIssue>,
		) {
			let value = decimal_text(parent.$field);
			length(
				issues,
				$code,
				&format!("patientInformation.parents.{idx}.{}", $path),
				value.as_deref(),
				$max_length,
			);
		}
	};
}

parent_decimal_length!(d_10_4, "ICH.D.10.4.LENGTH.MAX", "weightKg", weight_kg, 6);
parent_decimal_length!(d_10_5, "ICH.D.10.5.LENGTH.MAX", "heightCm", height_cm, 3);

/// ICH.D.10.5: parent height is a rounded integer.
fn d_10_5_integer(
	idx: usize,
	parent: &ParentInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if parent.height_cm.is_some_and(|value| !is_integer(value)) {
		push_business_issue(
			issues,
			"ICH.D.10.5.INTEGER",
			format!("patientInformation.parents.{idx}.heightCm"),
			"Parent height must be a whole number",
		);
	}
}

/// ICH.D.10.6.REQUIRED
/// ICH.D.10.6.ALLOWED.VALUE
/// ICH.D.10.6.LENGTH.MAX
fn d_10_6(
	idx: usize,
	parent: &ParentInformation,
	has_child_payload: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("patientInformation.parents.{idx}.sex");
	let has_payload = has_text(parent.parent_identification.as_deref())
		|| has_text(parent.parent_identification_null_flavor.as_deref())
		|| parent.parent_birth_date.is_some()
		|| has_text(parent.parent_birth_date_null_flavor.as_deref())
		|| parent.parent_age.is_some()
		|| has_text(parent.parent_age_unit.as_deref())
		|| parent.last_menstrual_period_date.is_some()
		|| has_text(parent.last_menstrual_period_date_null_flavor.as_deref())
		|| parent.weight_kg.is_some()
		|| parent.height_cm.is_some()
		|| has_text(parent.medical_history_text.as_deref())
		|| has_child_payload;
	required_when(
		issues,
		"ICH.D.10.6.REQUIRED",
		&path,
		"[D.10.6] Parent sex is required when parent data is populated.",
		has_payload,
		has_text(parent.sex.as_deref())
			|| has_text(parent.sex_null_flavor.as_deref()),
	);
	allowed(
		issues,
		"ICH.D.10.6.ALLOWED.VALUE",
		&path,
		valid_code(parent.sex.as_deref(), &["1", "2"]),
	);
	length(
		issues,
		"ICH.D.10.6.LENGTH.MAX",
		&path,
		parent.sex.as_deref(),
		1,
	);
}

/// ICH.D.10.7.2.LENGTH.MAX
fn d_10_7_2(
	idx: usize,
	parent: &ParentInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	length(
		issues,
		"ICH.D.10.7.2.LENGTH.MAX",
		&format!("patientInformation.parents.{idx}.medicalHistoryText"),
		parent.medical_history_text.as_deref(),
		10000,
	);
}

/// ICH.D.10.7.1.r.1a.REQUIRED
/// ICH.D.10.7.1.r.1a.LENGTH.MAX
fn d_10_7_1_r_1a(
	parent_idx: usize,
	idx: usize,
	episode: &ParentMedicalHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraVersion"
	);
	required_when(
		issues,
		"ICH.D.10.7.1.r.1a.REQUIRED",
		&path,
		"[D.10.7.1.r.1a] Parent medical history MedDRA version is required when [D.10.7.1.r.1b] is provided.",
		has_text(episode.meddra_code.as_deref()),
		has_text(episode.meddra_version.as_deref()),
	);
	length(
		issues,
		"ICH.D.10.7.1.r.1a.LENGTH.MAX",
		&path,
		episode.meddra_version.as_deref(),
		4,
	);
}

/// ICH.D.10.7.1.r.1b.REQUIRED
/// ICH.D.10.7.1.r.1b.LENGTH.MAX
fn d_10_7_1_r_1b(
	parent_idx: usize,
	idx: usize,
	episode: &ParentMedicalHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraCode"
	);
	required_when(
		issues,
		"ICH.D.10.7.1.r.1b.REQUIRED",
		&path,
		"[D.10.7.1.r.1b] Parent medical history MedDRA code is required when [D.10.7.1.r.1a] is provided.",
		has_text(episode.meddra_version.as_deref()),
		has_text(episode.meddra_code.as_deref()),
	);
	length(
		issues,
		"ICH.D.10.7.1.r.1b.LENGTH.MAX",
		&path,
		episode.meddra_code.as_deref(),
		8,
	);
}

/// ICH.D.10.7.1.r.1a.ALLOWED.VALUE
/// ICH.D.10.7.1.r.1a.VOCABULARY
/// ICH.D.10.7.1.r.1b.ALLOWED.VALUE
/// ICH.D.10.7.1.r.1b.VOCABULARY
fn d_10_7_1_r_1_meddra(
	parent_idx: usize,
	idx: usize,
	episode: &ParentMedicalHistory,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	meddra(
		issues,
		&validation_ctx.vocabulary,
		"ICH.D.10.7.1.r.1a.ALLOWED.VALUE",
		"ICH.D.10.7.1.r.1b.ALLOWED.VALUE",
		"ICH.D.10.7.1.r.1a.VOCABULARY",
		"ICH.D.10.7.1.r.1b.VOCABULARY",
		format!(
			"patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraVersion"
		),
		format!(
			"patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraCode"
		),
		episode.meddra_version.as_deref(),
		episode.meddra_code.as_deref(),
	);
}

/// ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN
fn d_10_7_1_r(
	parent_idx: usize,
	idx: usize,
	episode: &ParentMedicalHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_future_date(
		issues,
		"ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN",
		&format!(
			"patientInformation.parents.{parent_idx}.medicalHistory.{idx}.dateRange"
		),
		SECTION,
		"[D.10.7.1.r.2/D.10.7.1.r.4] Parent medical history dates must not be later than today.",
		DateValues::Two(episode.start_date, episode.end_date),
	);
}

/// ICH.D.10.7.1.r.5.LENGTH.MAX
fn d_10_7_1_r_5(
	parent_idx: usize,
	idx: usize,
	episode: &ParentMedicalHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	length(
		issues,
		"ICH.D.10.7.1.r.5.LENGTH.MAX",
		&format!(
			"patientInformation.parents.{parent_idx}.medicalHistory.{idx}.comments"
		),
		episode.comments.as_deref(),
		2000,
	);
}

/// ICH.D.10.8.r.1.LENGTH.MAX
fn d_10_8_r_1(
	parent_idx: usize,
	idx: usize,
	drug: &ParentPastDrugHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	length(
		issues,
		"ICH.D.10.8.r.1.LENGTH.MAX",
		&format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.drugName"),
		drug.drug_name.as_deref(),
		250,
	);
}

/// ICH.D.10.8.r.2a.LENGTH.MAX
fn d_10_8_r_2a(
	parent_idx: usize,
	idx: usize,
	drug: &ParentPastDrugHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mpidVersion"
	);
	length(
		issues,
		"ICH.D.10.8.r.2a.LENGTH.MAX",
		&path,
		drug.mpid_version.as_deref(),
		10,
	);
}

/// ICH.D.10.8.r.2b.ALLOWED.VALUE
/// ICH.D.10.8.r.2b.LENGTH.MAX
fn d_10_8_r_2b(
	parent_idx: usize,
	idx: usize,
	drug: &ParentPastDrugHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	let path =
		format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mpid");
	allowed(
		issues,
		"ICH.D.10.8.r.2b.ALLOWED.VALUE",
		&path,
		valid_identifier(drug.mpid.as_deref(), 1000),
	);
	length(
		issues,
		"ICH.D.10.8.r.2b.LENGTH.MAX",
		&path,
		drug.mpid.as_deref(),
		1000,
	);
}

/// ICH.D.10.8.r.3a.LENGTH.MAX
fn d_10_8_r_3a(
	parent_idx: usize,
	idx: usize,
	drug: &ParentPastDrugHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.phpidVersion"
	);
	length(
		issues,
		"ICH.D.10.8.r.3a.LENGTH.MAX",
		&path,
		drug.phpid_version.as_deref(),
		10,
	);
}

/// ICH.D.10.8.r.3b.ALLOWED.VALUE
/// ICH.D.10.8.r.3b.LENGTH.MAX
fn d_10_8_r_3b(
	parent_idx: usize,
	idx: usize,
	drug: &ParentPastDrugHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	let path =
		format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.phpid");
	allowed(
		issues,
		"ICH.D.10.8.r.3b.ALLOWED.VALUE",
		&path,
		valid_identifier(drug.phpid.as_deref(), 250),
	);
	length(
		issues,
		"ICH.D.10.8.r.3b.LENGTH.MAX",
		&path,
		drug.phpid.as_deref(),
		250,
	);
}

macro_rules! parent_past_drug_meddra_field {
	($name:ident, $suffix:literal, $label:literal, $path:literal, $version:ident, $code:ident) => {
		#[doc = concat!("ICH.D.10.8.r.", $suffix, "a.REQUIRED")]
		#[doc = concat!("ICH.D.10.8.r.", $suffix, "a.LENGTH.MAX")]
		#[doc = concat!("ICH.D.10.8.r.", $suffix, "b.REQUIRED")]
		#[doc = concat!("ICH.D.10.8.r.", $suffix, "b.LENGTH.MAX")]
		#[doc = concat!("ICH.D.10.8.r.", $suffix, "a.ALLOWED.VALUE")]
		#[doc = concat!("ICH.D.10.8.r.", $suffix, "a.VOCABULARY")]
		#[doc = concat!("ICH.D.10.8.r.", $suffix, "b.ALLOWED.VALUE")]
		#[doc = concat!("ICH.D.10.8.r.", $suffix, "b.VOCABULARY")]
		fn $name(
			parent_idx: usize,
			idx: usize,
			drug: &ParentPastDrugHistory,
			validation_ctx: &ValidationContext,
			issues: &mut Vec<ValidationIssue>,
		) {
			let version_path = format!(
				"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.{}MeddraVersion",
				$path
			);
			let code_path = format!(
				"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.{}MeddraCode",
				$path
			);
			required_when(
				issues,
				concat!("ICH.D.10.8.r.", $suffix, "a.REQUIRED"),
				&version_path,
				concat!("[D.10.8.r.", $suffix, "a] ", $label, " MedDRA version is required when [D.10.8.r.", $suffix, "b] is provided."),
				has_text(drug.$code.as_deref()),
				has_text(drug.$version.as_deref()),
			);
			required_when(
				issues,
				concat!("ICH.D.10.8.r.", $suffix, "b.REQUIRED"),
				&code_path,
				concat!("[D.10.8.r.", $suffix, "b] ", $label, " MedDRA code is required when [D.10.8.r.", $suffix, "a] is provided."),
				has_text(drug.$version.as_deref()),
				has_text(drug.$code.as_deref()),
			);
			length(
				issues,
				concat!("ICH.D.10.8.r.", $suffix, "a.LENGTH.MAX"),
				&version_path,
				drug.$version.as_deref(),
				4,
			);
			length(
				issues,
				concat!("ICH.D.10.8.r.", $suffix, "b.LENGTH.MAX"),
				&code_path,
				drug.$code.as_deref(),
				8,
			);
			meddra(
				issues,
				&validation_ctx.vocabulary,
				concat!("ICH.D.10.8.r.", $suffix, "a.ALLOWED.VALUE"),
				concat!("ICH.D.10.8.r.", $suffix, "b.ALLOWED.VALUE"),
				concat!("ICH.D.10.8.r.", $suffix, "a.VOCABULARY"),
				concat!("ICH.D.10.8.r.", $suffix, "b.VOCABULARY"),
				version_path,
				code_path,
				drug.$version.as_deref(),
				drug.$code.as_deref(),
			);
		}
	};
}

parent_past_drug_meddra_field!(
	d_10_8_r_6,
	"6",
	"Parent past drug indication",
	"indication",
	indication_meddra_version,
	indication_meddra_code
);
parent_past_drug_meddra_field!(
	d_10_8_r_7,
	"7",
	"Parent past drug reaction",
	"reaction",
	reaction_meddra_version,
	reaction_meddra_code
);

/// ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN
/// ICH.D.10.8.MPID_PHPID.EXCLUSIVE
fn d_10_8_r(
	parent_idx: usize,
	idx: usize,
	drug: &ParentPastDrugHistory,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_future_date(
		issues,
		"ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN",
		&format!(
			"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.dateRange"
		),
		SECTION,
		"[D.10.8.r.4/D.10.8.r.5] Parent past drug dates must not be later than today.",
		DateValues::Two(drug.start_date, drug.end_date),
	);
	reject_when(
		issues,
		"ICH.D.10.8.MPID_PHPID.EXCLUSIVE",
		&format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mpid"),
		SECTION,
		"[D.10.8.r.2b/D.10.8.r.3b] Any given parent past drug entry may have either MPID or PhPID, but not both.",
		has_text(drug.mpid.as_deref()) && has_text(drug.phpid.as_deref()),
	);
}

/// FDA.D.11.r.1 / FDA.D.11
fn fda_d_11(
	patient: Option<&PatientInformation>,
	vaers: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	const PATH: &str = "patientInformation.raceCodes";
	let null_flavor = patient
		.and_then(|patient| patient.race_code_null_flavor.as_deref())
		.map(str::trim)
		.filter(|value| !value.is_empty());
	let present = patient.is_some_and(|patient| {
		patient
			.race_codes
			.iter()
			.any(|value| has_text(Some(value.as_str())))
	}) || null_flavor.is_some();
	required_field(
		issues,
		if vaers {
			"FDA.D.11.REQUIRED"
		} else {
			"FDA.D.11.r.1.REQUIRED"
		},
		PATH,
		"FDA requires patient race or an allowed null flavor.",
		present,
	);
	reject_when(
		issues,
		"FDA.D.11.NULLFLAVOR.ROUTE",
		"patientInformation.raceCodeNullFlavor",
		SECTION,
		"FDA D.11 null flavor is not allowed for this reporting route.",
		null_flavor.is_some_and(|value| {
			if vaers {
				!matches!(value, "MSK" | "UNK" | "OTH")
			} else {
				!matches!(value, "MSK" | "UNK" | "OTH" | "NA")
			}
		}),
	);
	if let Some(patient) = patient {
		for (index, value) in patient.race_codes.iter().enumerate() {
			reject_when(
				issues,
				"FDA.D.11.r.1.ALLOWED.VALUE",
				&format!("patientInformation.raceCodes.{index}"),
				SECTION,
				"FDA D.11 race code must use the FDA regional terminology.",
				!matches!(
					value.trim(),
					"C16352" | "C41259" | "C41260" | "C41219" | "C41261"
				),
			);
		}
	}
}

/// FDA.D.12
fn fda_d_12(
	patient: Option<&PatientInformation>,
	vaers: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let null_flavor = patient
		.and_then(|patient| patient.ethnicity_code_null_flavor.as_deref())
		.map(str::trim)
		.filter(|value| !value.is_empty());
	let present = patient
		.is_some_and(|patient| has_text(patient.ethnicity_code.as_deref()))
		|| null_flavor.is_some();
	required_field(
		issues,
		"FDA.D.12.REQUIRED",
		"patientInformation.ethnicityCode",
		"FDA requires patient ethnicity or an allowed null flavor.",
		present,
	);
	reject_when(
		issues,
		"FDA.D.12.NULLFLAVOR.ROUTE",
		"patientInformation.ethnicityCodeNullFlavor",
		SECTION,
		"FDA D.12 null flavor is not allowed for this reporting route.",
		null_flavor.is_some_and(|value| {
			if vaers {
				!matches!(value, "NI" | "MSK" | "UNK")
			} else {
				!matches!(value, "NI" | "MSK" | "UNK" | "NA")
			}
		}),
	);
}

fn is_fda_vaers(validation_ctx: &ValidationContext) -> bool {
	validation_ctx
		.message_header
		.as_ref()
		.is_some_and(|header| {
			[
				Some(header.message_receiver_identifier.as_str()),
				header.batch_receiver_identifier.as_deref(),
			]
			.into_iter()
			.flatten()
			.any(|value| {
				matches!(
					value.trim().to_ascii_uppercase().as_str(),
					"CBER_VAERS" | "CBER VAERS"
				)
			})
		})
}

/// FDA.D.1: linked IND study reports use the aggregate patient marker.
fn fda_d_1(
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	issues: &mut Vec<ValidationIssue>,
) {
	let receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|header| header.message_receiver_identifier.as_str());
	let aggregate = validation_ctx.safety_report.as_ref().is_some_and(|report| {
		report.report_type.as_deref().map(str::trim) == Some("2")
	}) && is_fda_ind_message_receiver(receiver)
		&& fda_ctx.is_some_and(|ctx| {
			ctx.studies
				.iter()
				.any(|study| has_text(study.fda_ind_number_occurred.as_deref()))
		}) && !validation_ctx.linked_report_numbers.is_empty();
	if aggregate
		&& validation_ctx.patient.as_ref().is_none_or(|patient| {
			patient.patient_initials.as_deref().map(str::trim) != Some("AGGREGATE")
		}) {
		crate::push_business_issue(
			issues,
			"FDA.W0010",
			"patientInformation.patientInitials",
			"Linked IND study reports should identify the patient as AGGREGATE",
		);
	}
}

/// FDA.D.2: VAERS reports require one patient age description.
// FDA.D.2.1 (age at vaccination) is not represented by the current case model.
fn fda_d_2(patient: Option<&PatientInformation>, issues: &mut Vec<ValidationIssue>) {
	let present = patient.is_some_and(|patient| {
		patient.birth_date.is_some()
			|| has_text(patient.birth_date_null_flavor.as_deref())
			|| patient.age_at_time_of_onset.is_some()
			|| patient.gestation_period.is_some()
			|| has_text(patient.age_group.as_deref())
	});
	if !present {
		push_business_issue(
			issues,
			"FDA.D.2.REQUIRED",
			"patientInformation.patientBirthDate",
			"VAERS reports require at least one patient age description",
		);
	}
}

fn fda_d_2_required(local_criteria_report_type: Option<&str>) -> bool {
	local_criteria_report_type.map(str::trim) != Some("5")
}

fn fda_d_9_1_required(
	report_type: Option<&str>,
	receiver: Option<&str>,
	death_reported: bool,
	death_date_present: bool,
) -> bool {
	report_type.map(str::trim) == Some("2")
		&& is_fda_premarket_message_receiver(receiver)
		&& death_reported
		&& !death_date_present
}

/// FDA.D.9.1: date of death is required when a reaction caused death.
fn fda_d_9_1(validation_ctx: &ValidationContext, issues: &mut Vec<ValidationIssue>) {
	let report_type = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|report| report.report_type.as_deref());
	let receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|header| header.message_receiver_identifier.as_str());
	let death_reported = validation_ctx
		.reactions
		.iter()
		.any(|reaction| reaction.criteria_death == Some(true));
	let death_date_present =
		validation_ctx.death_info.as_ref().is_some_and(|death| {
			death.date_of_death.is_some()
				|| has_text(death.date_of_death_null_flavor.as_deref())
		});
	if fda_d_9_1_required(report_type, receiver, death_reported, death_date_present)
	{
		push_business_issue(
			issues,
			"FDA.D.9.1.REQUIRED",
			"patientInformation.death.dateOfDeath",
			"Date of death is required when death is reported as a seriousness criterion",
		);
	}
}

/// FDA.D.11 / FDA.D.12: aggregate or unavailable patients use NA.
fn fda_d_11_d_12_na(
	patient: &PatientInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let initials = patient.patient_initials.as_deref().map(str::trim);
	let null_flavor = patient
		.patient_initials_null_flavor
		.as_deref()
		.map(str::trim);
	if matches!(initials, Some("AGGREGATE" | "SUMMARY"))
		|| matches!(null_flavor, Some("NA"))
	{
		if patient.race_code_null_flavor.as_deref().map(str::trim) != Some("NA") {
			crate::push_business_issue(
				issues,
				"FDA.W0003",
				"patientInformation.raceCodeNullFlavor",
				"Race should use null flavor NA for aggregate or unavailable patients",
			);
		}
		if patient.ethnicity_code_null_flavor.as_deref().map(str::trim) != Some("NA")
		{
			crate::push_business_issue(issues, "FDA.W0004", "patientInformation.ethnicityCodeNullFlavor", "Ethnicity should use null flavor NA for aggregate or unavailable patients");
		}
	}
}

/// MFDS.D.1.1.4 / D.2.2 / D.5: clinical-trial and compassionate-use fields.
fn mfds_d_ct_cu(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let study_number = validation_ctx.patient_identifiers.iter().any(|identifier| {
		!identifier.deleted
			&& identifier.identifier_type_code.trim() == "4"
			&& (has_text(identifier.identifier_value.as_deref())
				|| has_text(identifier.identifier_value_null_flavor.as_deref()))
	});
	if !study_number {
		push_business_issue(
			issues,
			"MFDS.D.1.1.4.REQUIRED",
			"patientInformation.patientStudyNumber",
			"Patient study number is required for CT/CU reports",
		);
	}
	let patient = validation_ctx.patient.as_ref();
	if patient.is_none_or(|patient| patient.age_at_time_of_onset.is_none()) {
		push_business_issue(
			issues,
			"MFDS.D.2.2a.REQUIRED",
			"patientInformation.ageAtTimeOfOnset",
			"Patient age is required for CT/CU reports",
		);
	}
	if patient.is_none_or(|patient| !has_text(patient.age_unit.as_deref())) {
		push_business_issue(
			issues,
			"MFDS.D.2.2b.REQUIRED",
			"patientInformation.ageUnit",
			"Patient age unit is required for CT/CU reports",
		);
	}
	if patient.is_none_or(|patient| {
		!has_text(patient.sex.as_deref())
			&& !has_text(patient.sex_null_flavor.as_deref())
	}) {
		push_business_issue(
			issues,
			"MFDS.D.5.REQUIRED",
			"patientInformation.sex",
			"Patient sex is required for CT/CU reports",
		);
	}
}

/// MFDS.D.8.r.1.KR.1b.VOCABULARY
/// MFDS.D.8.r.1.KR.1b.REQUIRED
fn mfds_d_8_r_1_kr_1b(
	idx: usize,
	past: &crate::PastDrugByCase,
	receiver_is_kr_or_fr: bool,
	vocabulary_receiver: Option<&str>,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path =
		format!("patientInformation.pastDrugHistory.{idx}.mfdsMedicinalProductId");
	reject_when(
		issues,
		"MFDS.D.8.r.1.KR.1b.VOCABULARY",
		&path,
		SECTION,
		"Dictionary receiver-specific vocabulary constraint.",
		!valid_mfds_product(
			&validation_ctx.vocabulary,
			vocabulary_receiver,
			past.mfds_medicinal_product_version.as_deref(),
			past.mfds_medicinal_product_id.as_deref(),
		),
	);
	reject_when(
		issues,
		"MFDS.D.8.r.1.KR.1b.REQUIRED",
		&path,
		SECTION,
		"MFDS requires past drug code [D.8.r.1.KR.1b] for KR/FR receiver authorities.",
		receiver_is_kr_or_fr
			&& !has_text(past.drug_name_null_flavor.as_deref())
			&& !has_text(past.mfds_medicinal_product_id.as_deref()),
	);
}

/// MFDS.D.8.r.1.KR.1a.REQUIRED
fn mfds_d_8_r_1_kr_1a(
	idx: usize,
	past: &crate::PastDrugByCase,
	receiver_is_fr: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_when(
		issues,
		"MFDS.D.8.r.1.KR.1a.REQUIRED",
		&format!(
			"patientInformation.pastDrugHistory.{idx}.mfdsMedicinalProductVersion"
		),
		SECTION,
		"MFDS requires past drug code version [D.8.r.1.KR.1a] for FR when [D.8.r.1.KR.1b] is provided.",
		receiver_is_fr
			&& has_text(past.mfds_medicinal_product_id.as_deref())
			&& !has_text(past.mfds_medicinal_product_version.as_deref()),
	);
}

/// MFDS.D.8.r.2a/b and D.8.r.3a/b companion rules.
fn mfds_d_8_identifier_companions(
	idx: usize,
	past: &crate::PastDrugByCase,
	issues: &mut Vec<ValidationIssue>,
) {
	for (code, field, missing) in [
		(
			"MFDS.D.8.r.2a.REQUIRED",
			"mpidVersion",
			has_text(past.mpid.as_deref())
				&& !has_text(past.mpid_version.as_deref()),
		),
		(
			"MFDS.D.8.r.2b.REQUIRED",
			"mpid",
			has_text(past.mpid_version.as_deref())
				&& !has_text(past.mpid.as_deref()),
		),
		(
			"MFDS.D.8.r.3a.REQUIRED",
			"phpidVersion",
			has_text(past.phpid.as_deref())
				&& !has_text(past.phpid_version.as_deref()),
		),
		(
			"MFDS.D.8.r.3b.REQUIRED",
			"phpid",
			has_text(past.phpid_version.as_deref())
				&& !has_text(past.phpid.as_deref()),
		),
	] {
		reject_when(
			issues,
			code,
			&format!("patientInformation.pastDrugHistory.{idx}.{field}"),
			SECTION,
			"MFDS requires each D.8 MPID/PhPID identifier and version as a pair.",
			missing,
		);
	}
}

/// MFDS.D.10.8.r.2b / D.10.8.r.3b reverse companion rules.
fn mfds_d_10_8_identifier_companions(
	parent_idx: usize,
	idx: usize,
	past: &crate::ParentPastDrugByCase,
	issues: &mut Vec<ValidationIssue>,
) {
	for (code, field, missing) in [
		(
			"MFDS.D.10.8.r.2b.REQUIRED",
			"mpid",
			has_text(past.mpid_version.as_deref())
				&& !has_text(past.mpid.as_deref()),
		),
		(
			"MFDS.D.10.8.r.3b.REQUIRED",
			"phpid",
			has_text(past.phpid_version.as_deref())
				&& !has_text(past.phpid.as_deref()),
		),
	] {
		reject_when(
			issues,
			code,
			&format!(
				"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.{field}"
			),
			SECTION,
			"MFDS requires each D.10.8 MPID/PhPID identifier and version as a pair.",
			missing,
		);
	}
}

/// MFDS.D.10.8.r.1.KR.1b.VOCABULARY
/// MFDS.D.10.8.r.1.KR.1b.REQUIRED
fn mfds_d_10_8_r_1_kr_1b(
	parent_idx: usize,
	idx: usize,
	past: &crate::ParentPastDrugByCase,
	receiver_is_kr_or_fr: bool,
	vocabulary_receiver: Option<&str>,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mfdsMedicinalProductId"
	);
	reject_when(
		issues,
		"MFDS.D.10.8.r.1.KR.1b.VOCABULARY",
		&path,
		SECTION,
		"Dictionary receiver-specific vocabulary constraint.",
		!valid_mfds_product(
			&validation_ctx.vocabulary,
			vocabulary_receiver,
			past.mfds_medicinal_product_version.as_deref(),
			past.mfds_medicinal_product_id.as_deref(),
		),
	);
	reject_when(
		issues,
		"MFDS.D.10.8.r.1.KR.1b.REQUIRED",
		&path,
		SECTION,
		"MFDS requires parent past drug code [D.10.8.r.1.KR.1b] for KR/FR receiver authorities.",
		receiver_is_kr_or_fr && !has_text(past.mfds_medicinal_product_id.as_deref()),
	);
}

/// MFDS.D.10.8.r.1.KR.1a.REQUIRED
fn mfds_d_10_8_r_1_kr_1a(
	parent_idx: usize,
	idx: usize,
	past: &crate::ParentPastDrugByCase,
	receiver_is_fr: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_when(
		issues,
		"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
		&format!(
			"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mfdsMedicinalProductVersion"
		),
		SECTION,
		"MFDS requires parent past drug code version [D.10.8.r.1.KR.1a] for FR when [D.10.8.r.1.KR.1b] is provided.",
		receiver_is_fr
			&& has_text(past.mfds_medicinal_product_id.as_deref())
			&& !has_text(past.mfds_medicinal_product_version.as_deref()),
	);
}

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) {
	collect_ich_issues(validation_ctx, issues);
	if authority != RegulatoryAuthority::Fda
		&& validation_ctx
			.patient
			.as_ref()
			.and_then(|patient| patient.patient_initials_null_flavor.as_deref())
			.map(str::trim)
			== Some("NA")
	{
		push_business_issue(
			issues,
			"ICH.D.1.NULLFLAVOR.ALLOWED",
			"patientInformation.patientInitialsNullFlavor",
			"nullFlavor NA for D.1 is an FDA-only regional value",
		);
	}
	if authority == RegulatoryAuthority::Fda {
		collect_fda_issues(validation_ctx, fda_ctx, issues);
	}
	if authority == RegulatoryAuthority::Mfds {
		if let Some(mfds_ctx) = mfds_ctx {
			collect_mfds_issues(validation_ctx, mfds_ctx, issues);
		}
	}
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	d_1(validation_ctx.patient.as_ref(), issues);
	d_1_1_4(&validation_ctx.patient_identifiers, issues);
	for identifier in &validation_ctx.patient_identifiers {
		d_1_1_1(identifier, issues);
		d_1_1_2(identifier, issues);
		d_1_1_3(identifier, issues);
	}
	if let Some(patient) = validation_ctx.patient.as_ref() {
		d_2(patient, issues);
		d_2_1(patient, issues);
		d_2_2a(patient, issues);
		d_2_2b(patient, issues);
		d_2_2_1a(patient, issues);
		d_2_2_1b(patient, issues);
		d_2_3(patient, issues);
		d_3(patient, issues);
		d_4(patient, issues);
		d_4_integer(patient, issues);
		d_5(patient, issues);
		d_6(patient, issues);
		d_7_2(patient, validation_ctx.medical_history.is_empty(), issues);
		d_7_3(patient, issues);
	}
	for (idx, episode) in validation_ctx.medical_history.iter().enumerate() {
		d_7_1_r_1a(idx, episode, issues);
		d_7_1_r_1b(idx, episode, issues);
		d_7_1_r_1_meddra(idx, episode, validation_ctx, issues);
		d_7_1_r(idx, episode, issues);
		d_7_1_r_5(idx, episode, issues);
		d_7_1_r_6(idx, episode, issues);
		d_7_1_r_6_parent_duplicate(
			idx,
			episode,
			&validation_ctx.parent_medical_history,
			issues,
		);
	}
	for (idx, drug) in validation_ctx.past_drugs.iter().enumerate() {
		d_8_r_1(idx, drug, issues);
		d_8_r_2a(idx, drug, issues);
		d_8_r_2b(idx, drug, issues);
		d_8_r_3a(idx, drug, issues);
		d_8_r_3b(idx, drug, issues);
		d_8_r_6(idx, drug, validation_ctx, issues);
		d_8_r_7(idx, drug, validation_ctx, issues);
		d_8_r(idx, drug, issues);
	}
	for (idx, cause) in validation_ctx.reported_causes_of_death.iter().enumerate() {
		d_9_2_r_1a(idx, cause, issues);
		d_9_2_r_1b(idx, cause, issues);
		d_9_2_r_2(idx, cause, issues);
		d_9_2_r_1_meddra(idx, cause, validation_ctx, issues);
	}
	if let Some(death) = validation_ctx.death_info.as_ref() {
		d_9_1(death, issues);
		d_9_3(death, issues);
	}
	for (idx, cause) in validation_ctx.autopsy_causes_of_death.iter().enumerate() {
		d_9_4_r_1a(idx, cause, issues);
		d_9_4_r_1b(idx, cause, issues);
		d_9_4_r_2(idx, cause, issues);
		d_9_4_r_1_meddra(idx, cause, validation_ctx, issues);
	}
	for (idx, parent) in validation_ctx.parents.iter().enumerate() {
		d_10_1(idx, parent, issues);
		d_10_2_1(idx, parent, issues);
		d_10_2_2a(idx, parent, issues);
		d_10_2_2b(idx, parent, issues);
		d_10_2(idx, parent, issues);
		d_10_3(idx, parent, issues);
		d_10_4(idx, parent, issues);
		d_10_5(idx, parent, issues);
		d_10_5_integer(idx, parent, issues);
		let has_child_payload = validation_ctx
			.parent_medical_history
			.iter()
			.any(|episode| episode.parent_id == parent.id)
			|| validation_ctx
				.parent_past_drugs
				.iter()
				.any(|drug| drug.parent_id == parent.id);
		d_10_6(idx, parent, has_child_payload, issues);
		d_10_7_2(idx, parent, issues);
	}

	let parent_indices = parent_index_by_id(&validation_ctx.parents);
	let mut fallback_by_parent = HashMap::<Uuid, usize>::new();
	for episode in &validation_ctx.parent_medical_history {
		let Some(parent_idx) = parent_indices.get(&episode.parent_id).copied()
		else {
			continue;
		};
		let fallback = fallback_by_parent.entry(episode.parent_id).or_insert(0);
		let idx = *fallback;
		*fallback += 1;
		d_10_7_1_r_1a(parent_idx, idx, episode, issues);
		d_10_7_1_r_1b(parent_idx, idx, episode, issues);
		d_10_7_1_r_1_meddra(parent_idx, idx, episode, validation_ctx, issues);
		d_10_7_1_r(parent_idx, idx, episode, issues);
		d_10_7_1_r_5(parent_idx, idx, episode, issues);
	}

	let mut fallback_by_parent = HashMap::<Uuid, usize>::new();
	for drug in &validation_ctx.parent_past_drugs {
		let Some(parent_idx) = parent_indices.get(&drug.parent_id).copied() else {
			continue;
		};
		let fallback = fallback_by_parent.entry(drug.parent_id).or_insert(0);
		let idx = *fallback;
		*fallback += 1;
		d_10_8_r_1(parent_idx, idx, drug, issues);
		d_10_8_r_2a(parent_idx, idx, drug, issues);
		d_10_8_r_2b(parent_idx, idx, drug, issues);
		d_10_8_r_3a(parent_idx, idx, drug, issues);
		d_10_8_r_3b(parent_idx, idx, drug, issues);
		d_10_8_r_6(parent_idx, idx, drug, validation_ctx, issues);
		d_10_8_r_7(parent_idx, idx, drug, validation_ctx, issues);
		d_10_8_r(parent_idx, idx, drug, issues);
	}
}
pub(crate) fn collect_fda_issues(
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	issues: &mut Vec<ValidationIssue>,
) {
	fda_d_1(validation_ctx, fda_ctx, issues);
	fda_d_9_1(validation_ctx, issues);
	let vaers = is_fda_vaers(validation_ctx);
	if !vaers {
		if let Some(patient) = validation_ctx.patient.as_ref() {
			fda_d_11_d_12_na(patient, issues);
		}
	}
	let local_criteria = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|report| report.local_criteria_report_type.as_deref());
	if vaers && fda_d_2_required(local_criteria) {
		fda_d_2(validation_ctx.patient.as_ref(), issues);
	}
	if !vaers || local_criteria.map(str::trim) != Some("5") {
		fda_d_11(validation_ctx.patient.as_ref(), vaers, issues);
		fda_d_12(validation_ctx.patient.as_ref(), vaers, issues);
	}
}
pub(crate) fn collect_mfds_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	mfds_d_2_3(validation_ctx.patient.as_ref(), issues);
	let msg_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|header| header.message_receiver_identifier.as_str());
	let receiver_is_kr = is_mfds_domestic_receiver(msg_receiver);
	let receiver_is_fr = is_mfds_foreign_postmarket_receiver(msg_receiver);
	let receiver_is_ct_or_cu = is_mfds_clinical_trial_receiver(msg_receiver)
		|| is_mfds_compassionate_use_receiver(msg_receiver);
	if receiver_is_ct_or_cu {
		mfds_d_ct_cu(validation_ctx, issues);
	}
	let vocabulary_receiver = receiver_is_kr
		.then_some("KR")
		.or_else(|| receiver_is_fr.then_some("FR"));

	for (idx, past) in mfds_ctx.past_drugs.iter().enumerate() {
		mfds_d_8_identifier_companions(idx, past, issues);
		mfds_d_8_r_1_kr_1b(
			idx,
			past,
			receiver_is_kr || receiver_is_fr,
			vocabulary_receiver,
			validation_ctx,
			issues,
		);
		mfds_d_8_r_1_kr_1a(idx, past, receiver_is_fr, issues);
	}

	let parent_indices = parent_index_by_id(&validation_ctx.parents);
	let mut next_by_parent = HashMap::new();
	for past in &mfds_ctx.parent_past_drugs {
		let Some(parent_idx) = parent_indices.get(&past.parent_id).copied() else {
			continue;
		};
		let next = next_by_parent.entry(past.parent_id).or_insert(0);
		let idx = *next;
		*next += 1;
		mfds_d_10_8_r_1_kr_1b(
			parent_idx,
			idx,
			past,
			receiver_is_kr || receiver_is_fr,
			vocabulary_receiver,
			validation_ctx,
			issues,
		);
		mfds_d_10_8_identifier_companions(parent_idx, idx, past, issues);
		mfds_d_10_8_r_1_kr_1a(parent_idx, idx, past, receiver_is_fr, issues);
	}
}
#[cfg(test)]
#[path = "d/rule_tests.rs"]
mod rule_tests;

#[cfg(test)]
#[path = "d/golden_companion_tests.rs"]
mod golden_companion_tests;
