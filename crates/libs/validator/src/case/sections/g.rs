use super::helpers::{
	max_length, reject_future_date, reject_when, require, valid_code, valid_decimal,
	valid_dotted_version, valid_identifier, valid_iso3166, valid_meddra_term,
	valid_meddra_version, valid_mfds_product, valid_mfds_substance, valid_ucum,
	DateValues,
};
use crate::{
	has_text, is_fda_postmarket_batch_receiver, is_fda_premarket_message_receiver,
	is_mfds_clinical_trial_receiver, is_mfds_compassionate_use_receiver,
	is_mfds_domestic_receiver, is_mfds_foreign_postmarket_receiver,
	list_fda_devices, FdaValidationContext, MfdsValidationContext,
	RegulatoryAuthority, ValidationContext, ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::drug::{
	parse_drug_additional_info_codes_json, DosageInformation, DrugActiveSubstance,
	DrugIndication, DrugInformation, FdaDeviceCode, FdaDeviceInformation,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use lib_core::model::{ModelManager, Result};
use sqlx::types::{Decimal, Uuid};
use std::collections::{HashMap, HashSet};

const SECTION: &str = "drugs";
const MAX_LENGTH_MESSAGE: &str = "Dictionary max length exceeded.";
const ALLOWED_VALUE_MESSAGE: &str = "Dictionary allowed values constraint.";
const VOCABULARY_MESSAGE: &str = "Dictionary vocabulary constraint.";

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

fn decimal_text(value: Option<Decimal>) -> Option<String> {
	value.map(|value| value.normalize().to_string())
}

fn additional_info_codes(drug: &DrugInformation) -> Vec<String> {
	parse_drug_additional_info_codes_json(
		drug.drug_additional_info_codes_json.as_ref(),
	)
	.into_iter()
	.filter_map(|entry| entry.value_code)
	.collect()
}

/// ICH.G.k.1.REQUIRED
/// ICH.G.k.1.ALLOWED.VALUE
/// ICH.G.k.1.LENGTH.MAX
fn g_k_1(drugs: &[DrugInformation], issues: &mut Vec<ValidationIssue>) {
	required_field(
		issues,
		"ICH.G.k.REQUIRED",
		"drugs",
		"At least one drug is required.",
		!drugs.is_empty(),
	);
	for (idx, drug) in drugs.iter().enumerate() {
		let path = format!("drugs.{idx}.drugCharacterization");
		required_field(
			issues,
			"ICH.G.k.1.REQUIRED",
			&path,
			"[G.k.1] is required.",
			has_text(Some(drug.drug_characterization.as_str())),
		);
		allowed(
			issues,
			"ICH.G.k.1.ALLOWED.VALUE",
			&path,
			valid_code(
				Some(drug.drug_characterization.as_str()),
				&["1", "2", "3", "4"],
			),
		);
		length(
			issues,
			"ICH.G.k.1.LENGTH.MAX",
			&path,
			Some(drug.drug_characterization.as_str()),
			1,
		);
	}
	if !drugs.is_empty()
		&& !drugs
			.iter()
			.any(|drug| matches!(drug.drug_characterization.trim(), "1" | "3" | "4"))
	{
		crate::push_business_issue(
			issues,
			"ICH.G.k.1.AGGREGATE.REQUIRED",
			"drugs.0.drugCharacterization",
			"At least one drug must be Suspect, Interacting, or Drug Not Administered.",
		);
	}
}

/// ICH.G.k.2.1.1a.LENGTH.MAX
fn g_k_2_1_1a(
	idx: usize,
	drug: &DrugInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{idx}.mpidVersion");
	length(
		issues,
		"ICH.G.k.2.1.1a.LENGTH.MAX",
		&path,
		drug.mpid_version.as_deref(),
		10,
	);
}

/// ICH.G.k.2.1.1b.ALLOWED.VALUE
/// ICH.G.k.2.1.1b.LENGTH.MAX
fn g_k_2_1_1b(
	idx: usize,
	drug: &DrugInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{idx}.mpid");
	allowed(
		issues,
		"ICH.G.k.2.1.1b.ALLOWED.VALUE",
		&path,
		valid_identifier(drug.mpid.as_deref(), 1000),
	);
	length(
		issues,
		"ICH.G.k.2.1.1b.LENGTH.MAX",
		&path,
		drug.mpid.as_deref(),
		1000,
	);
}

/// ICH.G.k.2.1.2a.LENGTH.MAX
fn g_k_2_1_2a(
	idx: usize,
	drug: &DrugInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{idx}.phpidVersion");
	length(
		issues,
		"ICH.G.k.2.1.2a.LENGTH.MAX",
		&path,
		drug.phpid_version.as_deref(),
		10,
	);
}

/// ICH.G.k.2.1.2b.ALLOWED.VALUE
/// ICH.G.k.2.1.2b.LENGTH.MAX
fn g_k_2_1_2b(
	idx: usize,
	drug: &DrugInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{idx}.phpid");
	allowed(
		issues,
		"ICH.G.k.2.1.2b.ALLOWED.VALUE",
		&path,
		valid_identifier(drug.phpid.as_deref(), 250),
	);
	length(
		issues,
		"ICH.G.k.2.1.2b.LENGTH.MAX",
		&path,
		drug.phpid.as_deref(),
		250,
	);
	if has_text(drug.mpid.as_deref()) && has_text(drug.phpid.as_deref()) {
		crate::push_business_issue(
			issues,
			"ICH.G.k.2.1.MPID_PHPID.EXCLUSIVE",
			&path,
			"A drug may contain MPID or PhPID, but not both.",
		);
	}
}

/// ICH.G.k.2.2.REQUIRED
/// ICH.G.k.2.2.LENGTH.MAX
fn g_k_2_2(drugs: &[DrugInformation], issues: &mut Vec<ValidationIssue>) {
	for (idx, drug) in drugs.iter().enumerate() {
		let path = format!("drugs.{idx}.medicinalProduct");
		let value = Some(drug.medicinal_product.as_str());
		required_field(
			issues,
			"ICH.G.k.2.2.REQUIRED",
			&path,
			"[G.k.2.2] is required.",
			has_text(value),
		);
		length(issues, "ICH.G.k.2.2.LENGTH.MAX", &path, value, 250);
	}
}

/// ICH.G.k.2.4.VOCABULARY
/// ICH.G.k.2.4.LENGTH.MAX
fn g_k_2_4(
	idx: usize,
	drug: &DrugInformation,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{idx}.obtainDrugCountry");
	vocabulary(
		issues,
		"ICH.G.k.2.4.VOCABULARY",
		&path,
		valid_iso3166(vocabulary_ctx, drug.obtain_drug_country.as_deref()),
	);
	length(
		issues,
		"ICH.G.k.2.4.LENGTH.MAX",
		&path,
		drug.obtain_drug_country.as_deref(),
		2,
	);
}

/// ICH.G.k.2.5.ALLOWED.VALUE
fn g_k_2_5(
	idx: usize,
	drug: &DrugInformation,
	report_type_is_study: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{idx}.investigationalProductBlinded");
	allowed(
		issues,
		"ICH.G.k.2.5.ALLOWED.VALUE",
		&path,
		drug.investigational_product_blinded != Some(false),
	);
	if drug.investigational_product_blinded.is_some() && !report_type_is_study {
		crate::push_business_issue(
			issues,
			"ICH.G.k.2.5.STUDY.ONLY",
			&path,
			"Investigational Product Blinded must only be sent for a clinical-trial report.",
		);
	}
}

/// ICH.G.k.2.3.r.REQUIRED
fn g_k_2_3_r(
	drug_idx: usize,
	drug: &DrugInformation,
	substances: &[DrugActiveSubstance],
	issues: &mut Vec<ValidationIssue>,
) {
	if !has_text(drug.mpid.as_deref())
		&& !has_text(drug.phpid.as_deref())
		&& !substances.iter().any(|substance| {
			substance.drug_id == drug.id
				&& (has_text(substance.substance_name.as_deref())
					|| has_text(substance.substance_termid.as_deref()))
		}) {
		crate::push_business_issue(
			issues,
			"ICH.G.k.2.3.r.REQUIRED",
			if substances.iter().any(|row| row.drug_id == drug.id) {
				format!("drugs.{drug_idx}.activeSubstances.0.substanceName")
			} else {
				format!("drugs.{drug_idx}.activeSubstances")
			},
			"At least one active substance is required.",
		);
	}
}

/// ICH.G.k.3.1.LENGTH.MAX
fn g_k_3_1(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.drugAuthorizationNumber");
	length(
		issues,
		"ICH.G.k.3.1.LENGTH.MAX",
		&path,
		drug.drug_authorization_number.as_deref(),
		35,
	);
}

/// ICH.G.k.3.2.REQUIRED
/// ICH.G.k.3.2.VOCABULARY
/// ICH.G.k.3.2.LENGTH.MAX
fn g_k_3_2(
	idx: usize,
	drug: &DrugInformation,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{idx}.drugAuthorizationCountry");
	reject_when(
		issues,
		"ICH.G.k.3.2.REQUIRED",
		&path,
		SECTION,
		"[G.k.3.2] is required.",
		has_text(drug.drug_authorization_number.as_deref())
			&& !has_text(drug.manufacturer_country.as_deref()),
	);
	vocabulary(
		issues,
		"ICH.G.k.3.2.VOCABULARY",
		&path,
		valid_iso3166(vocabulary_ctx, drug.manufacturer_country.as_deref()),
	);
	length(
		issues,
		"ICH.G.k.3.2.LENGTH.MAX",
		&path,
		drug.manufacturer_country.as_deref(),
		2,
	);
}

/// ICH.G.k.3.3.LENGTH.MAX
fn g_k_3_3(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.manufacturerName");
	length(
		issues,
		"ICH.G.k.3.3.LENGTH.MAX",
		&path,
		drug.manufacturer_name.as_deref(),
		60,
	);
}

/// ICH.G.k.5a.REQUIRED
/// ICH.G.k.5a.LENGTH.MAX
fn g_k_5a(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.cumulativeDoseFirstReactionValue");
	reject_when(
		issues,
		"ICH.G.k.5a.REQUIRED",
		&path,
		SECTION,
		"[G.k.5a] Cumulative dose to first reaction value is required when [G.k.5b] is provided.",
		has_text(drug.cumulative_dose_first_reaction_unit.as_deref())
			&& drug.cumulative_dose_first_reaction_value.is_none(),
	);
	let value = decimal_text(drug.cumulative_dose_first_reaction_value);
	length(issues, "ICH.G.k.5a.LENGTH.MAX", &path, value.as_deref(), 10);
}

/// ICH.G.k.5b.REQUIRED
/// ICH.G.k.5b.LENGTH.MAX
fn g_k_5b(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.cumulativeDoseFirstReactionUnit");
	reject_when(
		issues,
		"ICH.G.k.5b.REQUIRED",
		&path,
		SECTION,
		"[G.k.5b] Cumulative dose to first reaction unit is required when [G.k.5a] is provided.",
		drug.cumulative_dose_first_reaction_value.is_some()
			&& !has_text(drug.cumulative_dose_first_reaction_unit.as_deref()),
	);
	length(
		issues,
		"ICH.G.k.5b.LENGTH.MAX",
		&path,
		drug.cumulative_dose_first_reaction_unit.as_deref(),
		50,
	);
}

/// ICH.G.k.6a.REQUIRED
/// ICH.G.k.6a.LENGTH.MAX
fn g_k_6a(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.gestationPeriodExposureValue");
	reject_when(
		issues,
		"ICH.G.k.6a.REQUIRED",
		&path,
		SECTION,
		"[G.k.6a] Gestation period at exposure value is required when [G.k.6b] is provided.",
		has_text(drug.gestation_period_exposure_unit.as_deref())
			&& drug.gestation_period_exposure_value.is_none(),
	);
	let value = decimal_text(drug.gestation_period_exposure_value);
	length(issues, "ICH.G.k.6a.LENGTH.MAX", &path, value.as_deref(), 3);
}

/// ICH.G.k.6b.REQUIRED
/// ICH.G.k.6b.LENGTH.MAX
fn g_k_6b(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.gestationPeriodExposureUnit");
	reject_when(
		issues,
		"ICH.G.k.6b.REQUIRED",
		&path,
		SECTION,
		"[G.k.6b] Gestation period at exposure unit is required when [G.k.6a] is provided.",
		drug.gestation_period_exposure_value.is_some()
			&& !has_text(drug.gestation_period_exposure_unit.as_deref()),
	);
	length(
		issues,
		"ICH.G.k.6b.LENGTH.MAX",
		&path,
		drug.gestation_period_exposure_unit.as_deref(),
		50,
	);
}

/// ICH.G.k.8.ALLOWED.VALUE
/// ICH.G.k.8.LENGTH.MAX
fn g_k_8(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.actionTaken");
	allowed(
		issues,
		"ICH.G.k.8.ALLOWED.VALUE",
		&path,
		valid_code(
			drug.action_taken.as_deref(),
			&["1", "2", "3", "4", "0", "9"],
		),
	);
	length(
		issues,
		"ICH.G.k.8.LENGTH.MAX",
		&path,
		drug.action_taken.as_deref(),
		1,
	);
}

/// ICH.G.k.10.r.ALLOWED.VALUE
/// ICH.G.k.10.r.LENGTH.MAX
fn g_k_10_r(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.drugAdditionalInformationCodes");
	let values = additional_info_codes(drug);
	allowed(
		issues,
		"ICH.G.k.10.r.ALLOWED.VALUE",
		&path,
		values.iter().all(|value| {
			matches!(
				value.trim(),
				"1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "10" | "11"
			)
		}),
	);
	let longest = values.iter().max_by_key(|value| value.chars().count());
	length(
		issues,
		"ICH.G.k.10.r.LENGTH.MAX",
		&path,
		longest.map(String::as_str),
		2,
	);
}

/// ICH.G.k.11.LENGTH.MAX
fn g_k_11(idx: usize, drug: &DrugInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("drugs.{idx}.drugAdditionalInformation");
	length(
		issues,
		"ICH.G.k.11.LENGTH.MAX",
		&path,
		drug.drug_additional_information.as_deref(),
		2000,
	);
}

/// ICH.G.k.2.3.r.1.REQUIRED
/// ICH.G.k.2.3.r.1.LENGTH.MAX
fn g_k_2_3_r_1(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	substance: &DrugActiveSubstance,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceName");
	reject_when(
		issues,
		"ICH.G.k.2.3.r.1.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.2.3.r.1] Substance name is required when an active substance row has no TermID.",
		!has_text(substance.substance_termid.as_deref())
			&& !has_text(substance.substance_name.as_deref()),
	);
	if let Some((drug_idx, idx)) = nested {
		let path = format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceName");
		length(
			issues,
			"ICH.G.k.2.3.r.1.LENGTH.MAX",
			&path,
			substance.substance_name.as_deref(),
			250,
		);
	}
}

/// ICH.G.k.2.3.r.2a.REQUIRED
/// ICH.G.k.2.3.r.2a.LENGTH.MAX
fn g_k_2_3_r_2a(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	substance: &DrugActiveSubstance,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceTermIdVersion");
	reject_when(
		issues,
		"ICH.G.k.2.3.r.2a.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.2.3.r.2a] Substance TermID version is required when [G.k.2.3.r.2b] TermID is populated.",
		has_text(substance.substance_termid.as_deref())
			&& !has_text(substance.substance_termid_version.as_deref()),
	);
	if let Some((drug_idx, idx)) = nested {
		let path = format!(
			"drugs.{drug_idx}.activeSubstances.{idx}.substanceTermIdVersion"
		);
		length(
			issues,
			"ICH.G.k.2.3.r.2a.LENGTH.MAX",
			&path,
			substance.substance_termid_version.as_deref(),
			10,
		);
	}
}

/// ICH.G.k.2.3.r.2b.ALLOWED.VALUE
/// ICH.G.k.2.3.r.2b.LENGTH.MAX
fn g_k_2_3_r_2b(
	nested: Option<(usize, usize)>,
	substance: &DrugActiveSubstance,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let path = format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceTermId");
	allowed(
		issues,
		"ICH.G.k.2.3.r.2b.ALLOWED.VALUE",
		&path,
		valid_identifier(substance.substance_termid.as_deref(), 250),
	);
	length(
		issues,
		"ICH.G.k.2.3.r.2b.LENGTH.MAX",
		&path,
		substance.substance_termid.as_deref(),
		100,
	);
}

/// ICH.G.k.2.3.r.3a.LENGTH.MAX
fn g_k_2_3_r_3a(
	nested: Option<(usize, usize)>,
	substance: &DrugActiveSubstance,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let path = format!("drugs.{drug_idx}.activeSubstances.{idx}.strengthValue");
	let value = decimal_text(substance.strength_value);
	length(
		issues,
		"ICH.G.k.2.3.r.3a.LENGTH.MAX",
		&path,
		value.as_deref(),
		10,
	);
}

/// ICH.G.k.2.3.r.3b.REQUIRED
/// ICH.G.k.2.3.r.3b.ALLOWED.VALUE
/// ICH.G.k.2.3.r.3b.LENGTH.MAX
fn g_k_2_3_r_3b(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	substance: &DrugActiveSubstance,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.activeSubstances.{idx}.strengthUnit");
	reject_when(
		issues,
		"ICH.G.k.2.3.r.3b.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.2.3.r.3b] Strength unit is required when [G.k.2.3.r.3a] is populated.",
		substance.strength_value.is_some()
			&& !has_text(substance.strength_unit.as_deref()),
	);
	if let Some((drug_idx, idx)) = nested {
		let path = format!("drugs.{drug_idx}.activeSubstances.{idx}.strengthUnit");
		allowed(
			issues,
			"ICH.G.k.2.3.r.3b.VOCABULARY",
			&path,
			valid_ucum(substance.strength_unit.as_deref()),
		);
		length(
			issues,
			"ICH.G.k.2.3.r.3b.LENGTH.MAX",
			&path,
			substance.strength_unit.as_deref(),
			50,
		);
	}
}

fn dosage_path(nested: Option<(usize, usize)>, field: &str) -> Option<String> {
	nested.map(|(drug_idx, idx)| {
		format!("drugs.{drug_idx}.dosageInformation.{idx}.{field}")
	})
}

/// ICH.G.k.4.r.1a.LENGTH.MAX
fn g_k_4_r_1a(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some(path) = dosage_path(nested, "doseValue") else {
		return;
	};
	let value = decimal_text(dosage.dose_value);
	length(
		issues,
		"ICH.G.k.4.r.1a.LENGTH.MAX",
		&path,
		value.as_deref(),
		8,
	);
}

/// ICH.G.k.4.r.1b.REQUIRED
/// ICH.G.k.4.r.1b.LENGTH.MAX
fn g_k_4_r_1b(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path = format!("drugs.{drug_idx}.dosageInformation.{idx}.doseUnit");
	reject_when(
		issues,
		"ICH.G.k.4.r.1b.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.4.r.1b] Dose unit is required when [G.k.4.r.1a] is populated.",
		dosage.dose_value.is_some() && !has_text(dosage.dose_unit.as_deref()),
	);
	if let Some(path) = dosage_path(nested, "doseUnit") {
		length(
			issues,
			"ICH.G.k.4.r.1b.LENGTH.MAX",
			&path,
			dosage.dose_unit.as_deref(),
			50,
		);
	}
}

/// ICH.G.k.4.r.2.LENGTH.MAX
fn g_k_4_r_2(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some(path) = dosage_path(nested, "numberOfUnits") else {
		return;
	};
	let value = decimal_text(dosage.number_of_units);
	length(
		issues,
		"ICH.G.k.4.r.2.LENGTH.MAX",
		&path,
		value.as_deref(),
		4,
	);
}

/// ICH.G.k.4.r.3.REQUIRED
/// ICH.G.k.4.r.3.ALLOWED.VALUE
/// ICH.G.k.4.r.3.LENGTH.MAX
fn g_k_4_r_3(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.dosageInformation.{idx}.frequencyUnit");
	reject_when(
		issues,
		"ICH.G.k.4.r.3.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.4.r.3] Time interval unit is required when [G.k.4.r.2] is populated.",
		dosage.number_of_units.is_some()
			&& !has_text(dosage.frequency_unit.as_deref()),
	);
	if let Some(path) = dosage_path(nested, "frequencyUnit") {
		allowed(
			issues,
			"ICH.G.k.4.r.3.VOCABULARY",
			&path,
			dosage
				.frequency_unit
				.as_deref()
				.map(str::trim)
				.filter(|value| !value.is_empty())
				.is_none_or(|value| {
					vocabulary_ctx.contains_snapshot_code(
						"ICH-UCUM",
						crate::VocabularyScope::Frequency,
						value,
					)
				}),
		);
		length(
			issues,
			"ICH.G.k.4.r.3.LENGTH.MAX",
			&path,
			dosage.frequency_unit.as_deref(),
			50,
		);
	}
}

/// ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN
fn g_k_4_r_4_5(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let start_path =
		format!("drugs.{drug_idx}.dosageInformation.{idx}.firstAdministrationDate");
	reject_future_date(
		issues,
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
		&start_path,
		SECTION,
		"[G.k.4.r.4/G.k.4.r.5] Drug administration dates must not be later than today.",
		DateValues::One(dosage.first_administration_date),
	);
	let end_path =
		format!("drugs.{drug_idx}.dosageInformation.{idx}.lastAdministrationDate");
	reject_future_date(
		issues,
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
		&end_path,
		SECTION,
		"[G.k.4.r.4/G.k.4.r.5] Drug administration dates must not be later than today.",
		DateValues::One(dosage.last_administration_date),
	);
}

/// ICH.G.k.4.r.6a.REQUIRED
/// ICH.G.k.4.r.6a.LENGTH.MAX
fn g_k_4_r_6a(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.dosageInformation.{idx}.durationValue");
	reject_when(
		issues,
		"ICH.G.k.4.r.6a.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.4.r.6a] Duration value is required when [G.k.4.r.6b] is populated.",
		has_text(dosage.duration_unit.as_deref()) && dosage.duration_value.is_none(),
	);
	if let Some(path) = dosage_path(nested, "durationValue") {
		let value = decimal_text(dosage.duration_value);
		length(
			issues,
			"ICH.G.k.4.r.6a.LENGTH.MAX",
			&path,
			value.as_deref(),
			5,
		);
	}
}

/// ICH.G.k.4.r.6b.REQUIRED
/// ICH.G.k.4.r.6b.LENGTH.MAX
fn g_k_4_r_6b(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.dosageInformation.{idx}.durationUnit");
	reject_when(
		issues,
		"ICH.G.k.4.r.6b.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.4.r.6b] Duration unit is required when [G.k.4.r.6a] is populated.",
		dosage.duration_value.is_some()
			&& !has_text(dosage.duration_unit.as_deref()),
	);
	if let Some(path) = dosage_path(nested, "durationUnit") {
		length(
			issues,
			"ICH.G.k.4.r.6b.LENGTH.MAX",
			&path,
			dosage.duration_unit.as_deref(),
			50,
		);
	}
}

/// ICH.G.k.4.r.7.LENGTH.MAX
fn g_k_4_r_7(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "batchLotNumber") {
		length(
			issues,
			"ICH.G.k.4.r.7.LENGTH.MAX",
			&path,
			dosage.batch_lot_number.as_deref(),
			35,
		);
	}
}
/// ICH.G.k.4.r.8.LENGTH.MAX
fn g_k_4_r_8(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "dosageText") {
		length(
			issues,
			"ICH.G.k.4.r.8.LENGTH.MAX",
			&path,
			dosage.dosage_text.as_deref(),
			2000,
		);
	}
}
/// ICH.G.k.4.r.9.1.LENGTH.MAX
fn g_k_4_r_9_1(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "doseForm") {
		length(
			issues,
			"ICH.G.k.4.r.9.1.LENGTH.MAX",
			&path,
			dosage.dose_form.as_deref(),
			60,
		);
	}
}

/// ICH.G.k.4.r.9.2a.REQUIRED
/// ICH.G.k.4.r.9.2a.LENGTH.MAX
fn g_k_4_r_9_2a(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.dosageInformation.{idx}.doseFormTermIdVersion");
	reject_when(
		issues,
		"ICH.G.k.4.r.9.2a.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.4.r.9.2a] Dose form TermID version is required when [G.k.4.r.9.2b] is populated.",
		has_text(dosage.dose_form_termid.as_deref())
			&& !has_text(dosage.dose_form_termid_version.as_deref()),
	);
	if let Some(path) = dosage_path(nested, "doseFormTermIdVersion") {
		length(
			issues,
			"ICH.G.k.4.r.9.2a.LENGTH.MAX",
			&path,
			dosage.dose_form_termid_version.as_deref(),
			10,
		);
	}
}
/// ICH.G.k.4.r.9.2b.LENGTH.MAX
fn g_k_4_r_9_2b(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "doseFormTermId") {
		length(
			issues,
			"ICH.G.k.4.r.9.2b.LENGTH.MAX",
			&path,
			dosage.dose_form_termid.as_deref(),
			100,
		);
	}
}
/// ICH.G.k.4.r.10.1.LENGTH.MAX
fn g_k_4_r_10_1(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "routeOfAdministration") {
		length(
			issues,
			"ICH.G.k.4.r.10.1.LENGTH.MAX",
			&path,
			dosage.route_of_administration.as_deref(),
			60,
		);
	}
}

/// ICH.G.k.4.r.10.2a.LENGTH.MAX
fn g_k_4_r_10_2a(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "routeTermIdVersion") {
		length(
			issues,
			"ICH.G.k.4.r.10.2a.LENGTH.MAX",
			&path,
			dosage.route_termid_version.as_deref(),
			10,
		);
	}
}
/// ICH.G.k.4.r.10.2b.LENGTH.MAX
fn g_k_4_r_10_2b(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "routeTermId") {
		length(
			issues,
			"ICH.G.k.4.r.10.2b.LENGTH.MAX",
			&path,
			dosage.route_termid.as_deref(),
			100,
		);
	}
}
/// ICH.G.k.4.r.11.1.LENGTH.MAX
fn g_k_4_r_11_1(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "parentRoute") {
		length(
			issues,
			"ICH.G.k.4.r.11.1.LENGTH.MAX",
			&path,
			dosage.parent_route.as_deref(),
			60,
		);
	}
}

/// ICH.G.k.4.r.11.2a.REQUIRED
/// ICH.G.k.4.r.11.2a.LENGTH.MAX
fn g_k_4_r_11_2a(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.dosageInformation.{idx}.parentRouteTermIdVersion");
	reject_when(
		issues,
		"ICH.G.k.4.r.11.2a.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.4.r.11.2a] Parent route TermID version is required when [G.k.4.r.11.2b] is populated.",
		has_text(dosage.parent_route_termid.as_deref())
			&& !has_text(dosage.parent_route_termid_version.as_deref()),
	);
	if let Some(path) = dosage_path(nested, "parentRouteTermIdVersion") {
		length(
			issues,
			"ICH.G.k.4.r.11.2a.LENGTH.MAX",
			&path,
			dosage.parent_route_termid_version.as_deref(),
			10,
		);
	}
}
/// ICH.G.k.4.r.11.2b.LENGTH.MAX
fn g_k_4_r_11_2b(
	nested: Option<(usize, usize)>,
	dosage: &DosageInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = dosage_path(nested, "parentRouteTermId") {
		length(
			issues,
			"ICH.G.k.4.r.11.2b.LENGTH.MAX",
			&path,
			dosage.parent_route_termid.as_deref(),
			100,
		);
	}
}

/// ICH.G.k.7.r.1.LENGTH.MAX
fn g_k_7_r_1(
	nested: Option<(usize, usize)>,
	indication: &DrugIndication,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let path = format!("drugs.{drug_idx}.indications.{idx}.indicationText");
	length(
		issues,
		"ICH.G.k.7.r.1.LENGTH.MAX",
		&path,
		indication.indication_text.as_deref(),
		250,
	);
}

/// ICH.G.k.7.r.2a.REQUIRED
/// ICH.G.k.7.r.2a.LENGTH.MAX
fn g_k_7_r_2a(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	indication: &DrugIndication,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraVersion");
	reject_when(
		issues,
		"ICH.G.k.7.r.2a.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.7.r.2a] Indication MedDRA version is required when [G.k.7.r.2b] is provided.",
		has_text(indication.indication_meddra_code.as_deref())
			&& !has_text(indication.indication_meddra_version.as_deref()),
	);
	if let Some((drug_idx, idx)) = nested {
		let path =
			format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraVersion");
		length(
			issues,
			"ICH.G.k.7.r.2a.LENGTH.MAX",
			&path,
			indication.indication_meddra_version.as_deref(),
			4,
		);
	}
}

/// ICH.G.k.7.r.2b.REQUIRED
/// ICH.G.k.7.r.2b.LENGTH.MAX
fn g_k_7_r_2b(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	indication: &DrugIndication,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path =
		format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraCode");
	reject_when(
		issues,
		"ICH.G.k.7.r.2b.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.7.r.2b] Indication MedDRA code is required when [G.k.7.r.2a] is provided.",
		has_text(indication.indication_meddra_version.as_deref())
			&& !has_text(indication.indication_meddra_code.as_deref()),
	);
	if let Some((drug_idx, idx)) = nested {
		let path =
			format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraCode");
		length(
			issues,
			"ICH.G.k.7.r.2b.LENGTH.MAX",
			&path,
			indication.indication_meddra_code.as_deref(),
			8,
		);
	}
}

/// ICH.G.k.7.r.2a.ALLOWED.VALUE
/// ICH.G.k.7.r.2a.VOCABULARY
/// ICH.G.k.7.r.2b.ALLOWED.VALUE
/// ICH.G.k.7.r.2b.VOCABULARY
fn g_k_7_r_2(
	nested: Option<(usize, usize)>,
	indication: &DrugIndication,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let version_path =
		format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraVersion");
	let code_path =
		format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraCode");
	let version = indication.indication_meddra_version.as_deref();
	let code = indication.indication_meddra_code.as_deref();
	allowed(
		issues,
		"ICH.G.k.7.r.2a.ALLOWED.VALUE",
		&version_path,
		valid_dotted_version(version),
	);
	allowed(
		issues,
		"ICH.G.k.7.r.2b.ALLOWED.VALUE",
		&code_path,
		valid_decimal(code),
	);
	vocabulary(
		issues,
		"ICH.G.k.7.r.2a.VOCABULARY",
		&version_path,
		valid_meddra_version(vocabulary_ctx, version),
	);
	vocabulary(
		issues,
		"ICH.G.k.7.r.2b.VOCABULARY",
		&code_path,
		valid_meddra_term(vocabulary_ctx, version, code),
	);
}

// The editor expands each assessment into its relatedness rows, retaining one
// empty row when no relatedness exists. All authorities use that same layout.
fn assessment_row_offsets(ctx: &ValidationContext) -> HashMap<Uuid, usize> {
	let mut counts = HashMap::new();
	for row in &ctx.relatedness_assessments {
		*counts
			.entry(row.drug_reaction_assessment_id)
			.or_insert(0usize) += 1;
	}
	let mut next_by_drug = HashMap::new();
	ctx.drug_reaction_assessments
		.iter()
		.map(|assessment| {
			let next = next_by_drug.entry(assessment.drug_id).or_insert(0);
			let offset = *next;
			*next += counts.get(&assessment.id).copied().unwrap_or(0).max(1);
			(assessment.id, offset)
		})
		.collect()
}

fn assessment_path(nested: Option<(usize, usize)>, field: &str) -> Option<String> {
	nested.map(|(drug_idx, idx)| {
		format!("drugs.{drug_idx}.drugReactionAssessments.{idx}.{field}")
	})
}

/// ICH.G.k.9.i.3.1a.REQUIRED
/// ICH.G.k.9.i.3.1a.LENGTH.MAX
fn g_k_9_i_3_1a(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	assessment: &DrugReactionAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path = format!(
		"drugs.{drug_idx}.drugReactionAssessments.{idx}.administrationStartIntervalValue"
	);
	reject_when(
		issues,
		"ICH.G.k.9.i.3.1a.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.9.i.3.1a] Administration start interval value is required when [G.k.9.i.3.1b] is populated.",
		has_text(assessment.administration_start_interval_unit.as_deref())
			&& assessment.administration_start_interval_value.is_none(),
	);
	if let Some(path) = assessment_path(nested, "administrationStartIntervalValue") {
		let value = decimal_text(assessment.administration_start_interval_value);
		length(
			issues,
			"ICH.G.k.9.i.3.1a.LENGTH.MAX",
			&path,
			value.as_deref(),
			5,
		);
	}
}

/// ICH.G.k.9.i.3.1b.REQUIRED
/// ICH.G.k.9.i.3.1b.LENGTH.MAX
fn g_k_9_i_3_1b(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	assessment: &DrugReactionAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path = format!(
		"drugs.{drug_idx}.drugReactionAssessments.{idx}.administrationStartIntervalUnit"
	);
	reject_when(
		issues,
		"ICH.G.k.9.i.3.1b.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.9.i.3.1b] Administration start interval unit is required when [G.k.9.i.3.1a] is populated.",
		assessment.administration_start_interval_value.is_some()
			&& !has_text(assessment.administration_start_interval_unit.as_deref()),
	);
	if let Some(path) = assessment_path(nested, "administrationStartIntervalUnit") {
		length(
			issues,
			"ICH.G.k.9.i.3.1b.LENGTH.MAX",
			&path,
			assessment.administration_start_interval_unit.as_deref(),
			50,
		);
	}
}

/// ICH.G.k.9.i.3.2a.REQUIRED
/// ICH.G.k.9.i.3.2a.LENGTH.MAX
fn g_k_9_i_3_2a(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	assessment: &DrugReactionAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path = format!(
		"drugs.{drug_idx}.drugReactionAssessments.{idx}.lastDoseIntervalValue"
	);
	reject_when(
		issues,
		"ICH.G.k.9.i.3.2a.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.9.i.3.2a] Last-dose interval value is required when [G.k.9.i.3.2b] is populated.",
		has_text(assessment.last_dose_interval_unit.as_deref())
			&& assessment.last_dose_interval_value.is_none(),
	);
	if let Some(path) = assessment_path(nested, "lastDoseIntervalValue") {
		let value = decimal_text(assessment.last_dose_interval_value);
		length(
			issues,
			"ICH.G.k.9.i.3.2a.LENGTH.MAX",
			&path,
			value.as_deref(),
			5,
		);
	}
}

/// ICH.G.k.9.i.3.2b.REQUIRED
/// ICH.G.k.9.i.3.2b.LENGTH.MAX
fn g_k_9_i_3_2b(
	_flat_idx: usize,
	nested: Option<(usize, usize)>,
	assessment: &DrugReactionAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some((drug_idx, idx)) = nested else {
		return;
	};
	let required_path = format!(
		"drugs.{drug_idx}.drugReactionAssessments.{idx}.lastDoseIntervalUnit"
	);
	reject_when(
		issues,
		"ICH.G.k.9.i.3.2b.REQUIRED",
		&required_path,
		SECTION,
		"[G.k.9.i.3.2b] Last-dose interval unit is required when [G.k.9.i.3.2a] is populated.",
		assessment.last_dose_interval_value.is_some()
			&& !has_text(assessment.last_dose_interval_unit.as_deref()),
	);
	if let Some(path) = assessment_path(nested, "lastDoseIntervalUnit") {
		length(
			issues,
			"ICH.G.k.9.i.3.2b.LENGTH.MAX",
			&path,
			assessment.last_dose_interval_unit.as_deref(),
			50,
		);
	}
}

/// ICH.G.k.9.i.4.ALLOWED.VALUE
/// ICH.G.k.9.i.4.LENGTH.MAX
fn g_k_9_i_4(
	nested: Option<(usize, usize)>,
	assessment: &DrugReactionAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some(path) = assessment_path(nested, "reactionRecurred") else {
		return;
	};
	allowed(
		issues,
		"ICH.G.k.9.i.4.ALLOWED.VALUE",
		&path,
		assessment
			.reaction_recurred
			.as_deref()
			.is_none_or(|value| matches!(value.trim(), "1" | "2" | "3" | "4")),
	);
	length(
		issues,
		"ICH.G.k.9.i.4.LENGTH.MAX",
		&path,
		assessment.reaction_recurred.as_deref(),
		1,
	);
}

/// CIOMS Item 20.ALLOWED.VALUE
fn cioms_item_20(
	nested: Option<(usize, usize)>,
	assessment: &DrugReactionAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some(path) = assessment_path(nested, "dechallengeResult") else {
		return;
	};
	allowed(
		issues,
		"CIOMS.ITEM20.ALLOWED.VALUE",
		&path,
		valid_code(assessment.dechallenge_result.as_deref(), &["1", "2", "3"]),
	);
}

fn relatedness_path(
	nested: Option<(usize, usize, usize)>,
	field: &str,
) -> Option<String> {
	nested.map(|(drug_idx, _assessment_idx, idx)| {
		format!("drugs.{drug_idx}.drugReactionAssessments.{idx}.{field}")
	})
}

/// ICH.G.k.9.i.2.r.1.LENGTH.MAX
fn g_k_9_i_2_r_1(
	nested: Option<(usize, usize, usize)>,
	relatedness: &RelatednessAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = relatedness_path(nested, "sourceOfAssessment") {
		length(
			issues,
			"ICH.G.k.9.i.2.r.1.LENGTH.MAX",
			&path,
			relatedness.source_of_assessment.as_deref(),
			60,
		);
	}
}
/// ICH.G.k.9.i.2.r.2.LENGTH.MAX
fn g_k_9_i_2_r_2(
	nested: Option<(usize, usize, usize)>,
	relatedness: &RelatednessAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = relatedness_path(nested, "methodOfAssessment") {
		length(
			issues,
			"ICH.G.k.9.i.2.r.2.LENGTH.MAX",
			&path,
			relatedness.method_of_assessment.as_deref(),
			60,
		);
	}
}
/// ICH.G.k.9.i.2.r.3.LENGTH.MAX
fn g_k_9_i_2_r_3(
	nested: Option<(usize, usize, usize)>,
	relatedness: &RelatednessAssessment,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(path) = relatedness_path(nested, "resultOfAssessment") {
		length(
			issues,
			"ICH.G.k.9.i.2.r.3.LENGTH.MAX",
			&path,
			relatedness.result_of_assessment.as_deref(),
			60,
		);
	}
}

pub(crate) async fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	mm: &ModelManager,
	ctx: &Ctx,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) -> Result<()> {
	collect_ich_issues(validation_ctx, issues);
	match authority {
		RegulatoryAuthority::Ich => {}
		RegulatoryAuthority::Fda => {
			collect_fda_issues(ctx, mm, validation_ctx, fda_ctx, issues).await?
		}
		RegulatoryAuthority::Mfds => {
			if let Some(mfds_ctx) = mfds_ctx {
				collect_mfds_issues(validation_ctx, mfds_ctx, issues);
			}
		}
	}
	Ok(())
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let is_clinical_trial = validation_ctx.studies.iter().any(|study| {
		study.study_type_reaction.as_deref().map(str::trim) == Some("1")
	});
	g_k_1(&validation_ctx.drugs, issues);
	g_k_2_2(&validation_ctx.drugs, issues);
	for (idx, drug) in validation_ctx.drugs.iter().enumerate() {
		g_k_2_1_1a(idx, drug, issues);
		g_k_2_1_1b(idx, drug, issues);
		g_k_2_1_2a(idx, drug, issues);
		g_k_2_1_2b(idx, drug, issues);
		g_k_2_4(idx, drug, &validation_ctx.vocabulary, issues);
		g_k_2_5(idx, drug, is_clinical_trial, issues);
		g_k_2_3_r(idx, drug, &validation_ctx.active_substances, issues);
		g_k_3_1(idx, drug, issues);
		g_k_3_2(idx, drug, &validation_ctx.vocabulary, issues);
		g_k_3_3(idx, drug, issues);
		g_k_5a(idx, drug, issues);
		g_k_5b(idx, drug, issues);
		g_k_6a(idx, drug, issues);
		g_k_6b(idx, drug, issues);
		g_k_8(idx, drug, issues);
		g_k_10_r(idx, drug, issues);
		g_k_11(idx, drug, issues);
	}

	let drug_indices = validation_ctx
		.drugs
		.iter()
		.enumerate()
		.map(|(idx, drug)| (drug.id, idx))
		.collect::<HashMap<_, _>>();
	let mut fallback = HashMap::new();
	for (flat_idx, substance) in validation_ctx.active_substances.iter().enumerate()
	{
		let Some(drug_idx) = drug_indices.get(&substance.drug_id).copied() else {
			continue;
		};
		let fallback_idx = fallback.entry(substance.drug_id).or_insert(0);
		let idx = *fallback_idx;
		*fallback_idx += 1;
		let nested = Some((drug_idx, idx));
		g_k_2_3_r_1(flat_idx, nested, substance, issues);
		g_k_2_3_r_2a(flat_idx, nested, substance, issues);
		g_k_2_3_r_2b(nested, substance, issues);
		g_k_2_3_r_3a(nested, substance, issues);
		g_k_2_3_r_3b(flat_idx, nested, substance, issues);
	}

	let mut fallback = HashMap::new();
	for (flat_idx, dosage) in validation_ctx.dosages.iter().enumerate() {
		let Some(drug_idx) = drug_indices.get(&dosage.drug_id).copied() else {
			continue;
		};
		let fallback_idx = fallback.entry(dosage.drug_id).or_insert(0);
		let idx = *fallback_idx;
		*fallback_idx += 1;
		let nested = Some((drug_idx, idx));
		g_k_4_r_1a(nested, dosage, issues);
		g_k_4_r_1b(flat_idx, nested, dosage, issues);
		g_k_4_r_2(nested, dosage, issues);
		g_k_4_r_3(flat_idx, nested, dosage, &validation_ctx.vocabulary, issues);
		g_k_4_r_4_5(nested, dosage, issues);
		g_k_4_r_6a(flat_idx, nested, dosage, issues);
		g_k_4_r_6b(flat_idx, nested, dosage, issues);
		g_k_4_r_7(nested, dosage, issues);
		g_k_4_r_8(nested, dosage, issues);
		g_k_4_r_9_1(nested, dosage, issues);
		g_k_4_r_9_2a(flat_idx, nested, dosage, issues);
		g_k_4_r_9_2b(nested, dosage, issues);
		g_k_4_r_10_1(nested, dosage, issues);
		g_k_4_r_10_2a(nested, dosage, issues);
		g_k_4_r_10_2b(nested, dosage, issues);
		g_k_4_r_11_1(nested, dosage, issues);
		g_k_4_r_11_2a(flat_idx, nested, dosage, issues);
		g_k_4_r_11_2b(nested, dosage, issues);
	}

	let mut fallback = HashMap::new();
	for (flat_idx, indication) in validation_ctx.indications.iter().enumerate() {
		let Some(drug_idx) = drug_indices.get(&indication.drug_id).copied() else {
			continue;
		};
		let fallback_idx = fallback.entry(indication.drug_id).or_insert(0);
		let idx = *fallback_idx;
		*fallback_idx += 1;
		let nested = Some((drug_idx, idx));
		g_k_7_r_1(nested, indication, issues);
		g_k_7_r_2a(flat_idx, nested, indication, issues);
		g_k_7_r_2b(flat_idx, nested, indication, issues);
		g_k_7_r_2(nested, indication, &validation_ctx.vocabulary, issues);
	}

	let relatedness_bases = assessment_row_offsets(validation_ctx);
	let mut assessment_indices = HashMap::new();
	for (flat_idx, assessment) in
		validation_ctx.drug_reaction_assessments.iter().enumerate()
	{
		let Some(drug_idx) = drug_indices.get(&assessment.drug_id).copied() else {
			continue;
		};
		let idx = relatedness_bases[&assessment.id];
		assessment_indices.insert(assessment.id, (drug_idx, idx));
		let nested = Some((drug_idx, idx));
		g_k_9_i_3_1a(flat_idx, nested, assessment, issues);
		g_k_9_i_3_1b(flat_idx, nested, assessment, issues);
		g_k_9_i_3_2a(flat_idx, nested, assessment, issues);
		g_k_9_i_3_2b(flat_idx, nested, assessment, issues);
		g_k_9_i_4(nested, assessment, issues);
		cioms_item_20(nested, assessment, issues);
	}
	let mut fallback = HashMap::new();
	for relatedness in &validation_ctx.relatedness_assessments {
		let assessment_id = relatedness.drug_reaction_assessment_id;
		let nested = assessment_indices.get(&assessment_id).copied().and_then(
			|(drug_idx, assessment_idx)| {
				let fallback_idx = fallback.entry(assessment_id).or_insert(0);
				let child_idx =
					relatedness_bases.get(&assessment_id).copied()? + *fallback_idx;
				*fallback_idx += 1;
				Some((drug_idx, assessment_idx, child_idx))
			},
		);
		g_k_9_i_2_r_1(nested, relatedness, issues);
		g_k_9_i_2_r_2(nested, relatedness, issues);
		g_k_9_i_2_r_3(nested, relatedness, issues);
	}
}

/// FDA.G.K.12.REQUIRED
fn fda_g_k_12(
	drug_idx: usize,
	local_criteria_is_malfunction_only: bool,
	has_suspect_malfunction: bool,
	has_device: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	if local_criteria_is_malfunction_only && !has_suspect_malfunction && !has_device
	{
		crate::push_business_issue(
			issues,
			"FDA.G.k.12.COLLECTION.REQUIRED",
			format!("drugs.{drug_idx}.fdaDevices"),
			"At least one device is required.",
		);
		return;
	}
	reject_when(
		issues,
		"FDA.G.K.12.REQUIRED",
		&format!("drugs.{drug_idx}.fdaDevices.0.malfunction"),
		SECTION,
		"FDA postmarket requires at least one suspect product with [G.K.12.r.1]=true when [C.1.7.1]=5.",
		local_criteria_is_malfunction_only && !has_suspect_malfunction,
	);
}

/// FDA.G.k.12.r.4-6
fn fda_g_k_12_r_4_6(
	drug_idx: usize,
	device_idx: usize,
	identity: [Option<&str>; 3],
	required: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	required_when(
		issues,
		"FDA.G.k.12.r.4-6.AT_LEAST_ONE",
		&format!("drugs.{drug_idx}.fdaDevices.{device_idx}.deviceBrandName"),
		"A required device must have a non-null brand name, common name, or product code.",
		required,
		identity.into_iter().any(has_text),
	);
}

fn fda_device_code_rules(
	drug_idx: usize,
	device_idx: usize,
	device: &FdaDeviceInformation,
	device_codes: &[FdaDeviceCode],
	local_criteria: Option<&str>,
	issues: &mut Vec<ValidationIssue>,
) {
	if device.malfunction != Some(true) {
		return;
	}
	let path = format!("drugs.{drug_idx}.fdaDevices.{device_idx}");
	let has_problem = device_codes.iter().any(|code| {
		code.device_id == device.id
			&& code.element == "device_problem"
			&& has_text(Some(&code.value_code))
	});
	if !has_problem {
		crate::push_business_issue(
			issues,
			"FDA.G.K.12.R.3.REQUIRED",
			if device_codes.iter().any(|code| {
				code.device_id == device.id && code.element == "device_problem"
			}) {
				format!("{path}.deviceProblemCodes.0.valueCode")
			} else {
				format!("{path}.deviceProblemCodes")
			},
			"A device problem code is required for each malfunctioning device.",
		);
	}
	let has_remedial = device_codes.iter().any(|code| {
		code.device_id == device.id
			&& code.element == "remedial_action"
			&& has_text(Some(&code.value_code))
	});
	if local_criteria == Some("4") && !has_remedial {
		crate::push_business_issue(
			issues,
			"FDA.W0007",
			if device_codes.iter().any(|code| {
				code.device_id == device.id && code.element == "remedial_action"
			}) {
				format!("{path}.remedialActions.0.valueCode")
			} else {
				format!("{path}.remedialActions")
			},
			"A remedial action should be provided for each malfunctioning device in a 5-day report.",
		);
	}
}

/// FDA.R0072
fn fda_g_k_1_a(
	drug_idx: usize,
	value: Option<&str>,
	combination_product: bool,
	malfunction: bool,
	drug_characterization: &str,
	required_when_allowed: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let allowed_context =
		combination_product && malfunction && drug_characterization.trim() == "4";
	let value = value.map(str::trim).filter(|value| !value.is_empty());
	reject_when(
		issues,
		"FDA.R0072",
		&format!("drugs.{drug_idx}.fdaOtherCharacterization"),
		SECTION,
		"FDA [G.k.1.a]=1 is required for AEMS, and only allowed for VAERS, when [C.1.12]=true, [G.k.12.r.1]=true, and [G.k.1]=4 for the same product.",
		(required_when_allowed && allowed_context && value != Some("1"))
			|| (!allowed_context && value == Some("1")),
	);
}

/// FDA.D.1 R0027: a combination-product malfunction without an adverse event
/// must identify the unavailable patient with nullFlavor NA.
fn fda_d_1_malfunction(
	validation_ctx: &ValidationContext,
	combination_true: bool,
	has_malfunction: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let malfunction_without_event =
		validation_ctx.reactions.iter().any(|reaction| {
			reaction.reaction_meddra_code.as_deref().map(str::trim)
				== Some("10067482")
		});
	if combination_true
		&& has_malfunction
		&& malfunction_without_event
		&& validation_ctx.patient.as_ref().is_none_or(|patient| {
			patient
				.patient_initials_null_flavor
				.as_deref()
				.map(str::trim)
				!= Some("NA")
		}) {
		crate::push_business_issue(
			issues,
			"FDA.D.1.R0027",
			"patientInformation.patientInitialsNullFlavor",
			"Patient identification must use null flavor NA for a combination-product malfunction without an adverse event.",
		);
	}
}

/// FDA.G.k.1.ROUTE
fn fda_g_k_1_route(
	validation_ctx: &ValidationContext,
	first_product_malfunction: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some(first) = validation_ctx.drugs.first() else {
		return;
	};
	let batch_receiver = validation_ctx
		.message_header
		.as_ref()
		.and_then(|header| header.batch_receiver_identifier.as_deref());
	let message_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|header| header.message_receiver_identifier.as_str());
	let role = first.drug_characterization.trim();
	if message_receiver == Some(crate::FDA_MSG_RECEIVER_CDER) {
		if !matches!(role, "1" | "3" | "4") {
			crate::push_business_issue(
				issues,
				"FDA.R0069",
				"drugs.0.drugCharacterization",
				"FDA postmarket drug characterization must be 1, 3, or 4.",
			);
		}
		return;
	}
	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|report| report.report_type.as_deref())
		.map(str::trim)
		== Some("2");
	if report_type_is_study {
		let rule = match message_receiver {
			Some(
				crate::FDA_MSG_RECEIVER_CDER_IND | crate::FDA_MSG_RECEIVER_CBER_IND,
			) => Some(("FDA.R0070", &["1", "3"][..])),
			Some(crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE) => {
				Some(("FDA.R0071", &["1", "3", "4"][..]))
			}
			_ => None,
		};
		if let Some((code, allowed)) = rule {
			for (idx, drug) in validation_ctx.drugs.iter().enumerate() {
				if !allowed.contains(&drug.drug_characterization.trim()) {
					crate::push_business_issue(
						issues,
						code,
						format!("drugs.{idx}.drugCharacterization"),
						"Drug characterization is not valid for the selected FDA route.",
					);
				}
			}
			return;
		}
	}
	let vaers = [batch_receiver, message_receiver]
		.into_iter()
		.flatten()
		.any(|receiver| receiver.to_ascii_uppercase().contains("VAERS"));
	let invalid = (is_fda_postmarket_batch_receiver(batch_receiver)
		&& !matches!(role, "1" | "3")
		&& !(first_product_malfunction && role == "4"))
		|| (vaers && !matches!(role, "1" | "3"));
	if invalid {
		crate::push_business_issue(
			issues,
			"FDA.G.k.1.ROUTE",
			"drugs.0.drugCharacterization",
			"Drug characterization is not valid for the selected FDA route.",
		);
	}
}

/// FDA.G.k.10a.REQUIRED
fn fda_g_k_10a(
	idx: usize,
	value: Option<&str>,
	null_flavor: Option<&str>,
	pre_anda_present: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	if !pre_anda_present {
		return;
	}
	let value = value.map(str::trim).filter(|value| !value.is_empty());
	let null_flavor = null_flavor.map(str::trim).filter(|value| !value.is_empty());
	let invalid = !matches!(value, Some("1" | "2")) && null_flavor != Some("NA");
	if invalid {
		crate::push_business_issue(
			issues,
			"FDA.W0006",
			format!("drugs.{idx}.fdaAdditionalInfoCoded"),
			"FDA.G.k.10a should be 1, 2, or null flavor NA for an IND-exempt BA/BE study.",
		);
	}
}

/// FDA.G.k.9.i.2.r.1.REQUIRED
/// FDA.G.k.9.i.2.r.2.REQUIRED
/// FDA.G.k.9.i.2.r.3.REQUIRED
fn fda_g_k_9(
	validation_ctx: &ValidationContext,
	ind_number_present: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|report| report.report_type.as_deref())
		.map(str::trim)
		== Some("2");
	if !report_type_is_study || !ind_number_present {
		return;
	}
	let drug_indices = validation_ctx
		.drugs
		.iter()
		.enumerate()
		.map(|(idx, drug)| (drug.id, idx))
		.collect::<HashMap<_, _>>();
	let first_suspect_idx = validation_ctx
		.drugs
		.iter()
		.position(|drug| drug.drug_characterization.trim() == "1");
	if validation_ctx.drug_reaction_assessments.is_empty() {
		let Some(drug_idx) = first_suspect_idx else {
			return;
		};
		crate::push_business_issue(issues, "FDA.G.k.9.REQUIRED",
			format!("drugs.{drug_idx}.drugReactionAssessments"),
			"At least one drug-reaction assessment is required for an IND safety report.");
		return;
	}
	let relatedness_bases = assessment_row_offsets(validation_ctx);
	let mut has_suspect_result = false;
	let mut first_suspect_result_path = None;
	let mut first_suspect_assessment_path = None;
	for assessment in &validation_ctx.drug_reaction_assessments {
		let Some(drug_idx) = drug_indices.get(&assessment.drug_id).copied() else {
			continue;
		};
		let is_suspect =
			validation_ctx.drugs[drug_idx].drug_characterization.trim() == "1";
		let Some(relatedness_base) = relatedness_bases.get(&assessment.id).copied()
		else {
			continue;
		};
		let rows = validation_ctx
			.relatedness_assessments
			.iter()
			.filter(|row| row.drug_reaction_assessment_id == assessment.id)
			.collect::<Vec<_>>();
		if is_suspect && first_suspect_assessment_path.is_none() {
			first_suspect_assessment_path = Some(format!(
				"drugs.{drug_idx}.drugReactionAssessments.{relatedness_base}.resultOfAssessment"
			));
		}
		for (fallback_idx, row) in rows.iter().enumerate() {
			let relatedness_idx = relatedness_base + fallback_idx;
			let has_source = row.source_of_assessment.as_deref().map(str::trim)
				== Some("Sponsor");
			let has_method =
				row.method_of_assessment.as_deref().map(str::trim) == Some("FDA");
			let has_result = matches!(
				row.result_of_assessment.as_deref().map(str::trim),
				Some("Suspected" | "Not Suspected")
			);
			if is_suspect {
				has_suspect_result |= has_result;
				if has_result && first_suspect_result_path.is_none() {
					first_suspect_result_path = Some(format!(
						"drugs.{drug_idx}.drugReactionAssessments.{relatedness_idx}.resultOfAssessment"
					));
				}
			}
			for (code, field, present, message) in [
				(
					"FDA.G.k.9.i.2.r.1.REQUIRED",
					"sourceOfAssessment",
					has_source,
					"Source of Assessment is required for an IND safety report.",
				),
				(
					"FDA.G.k.9.i.2.r.2.REQUIRED",
					"methodOfAssessment",
					has_method,
					"Method of Assessment is required for an IND safety report.",
				),
			] {
				if !present {
					crate::push_business_issue(
						issues,
						code,
						format!("drugs.{drug_idx}.drugReactionAssessments.{relatedness_idx}.{field}"),
						message,
					);
				}
			}
		}
		if rows.is_empty() {
			for (code, field) in [
				("FDA.G.k.9.i.2.r.1.REQUIRED", "sourceOfAssessment"),
				("FDA.G.k.9.i.2.r.2.REQUIRED", "methodOfAssessment"),
			] {
				crate::push_business_issue(
				issues,
				code,
				format!("drugs.{drug_idx}.drugReactionAssessments.{relatedness_base}.{field}"),
					"A relatedness assessment value is required for an IND safety report.",
				);
			}
		}
	}
	if !has_suspect_result {
		crate::push_business_issue(
			issues,
			"FDA.G.k.9.i.2.r.3.REQUIRED",
			first_suspect_result_path
				.or(first_suspect_assessment_path)
			.unwrap_or_else(|| "drugs".to_string()),
			"At least one suspect product must have a relatedness result for an IND safety report.",
		);
	}
}

pub(crate) async fn collect_fda_issues(
	ctx: &Ctx,
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	issues: &mut Vec<ValidationIssue>,
) -> Result<()> {
	let local_criteria = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|r| r.local_criteria_report_type.as_deref());
	let combination_true = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|r| r.combination_product_report_indicator.as_deref())
		.map(str::trim)
		.is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
	let message_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|header| header.message_receiver_identifier.as_str());
	let batch_receiver = validation_ctx
		.message_header
		.as_ref()
		.and_then(|header| header.batch_receiver_identifier.as_deref());
	let postmarket = is_fda_postmarket_batch_receiver(batch_receiver);
	let vaers = message_receiver.is_some_and(|value| {
		matches!(
			value.trim().to_ascii_uppercase().as_str(),
			"CBER_VAERS" | "CBER VAERS"
		)
	});
	let device_rules_apply = !is_fda_premarket_message_receiver(message_receiver);

	let mut has_malfunction_suspect = false;
	let mut has_malfunction = false;
	let mut first_product_malfunction = false;
	let mut has_device = false;
	let mut drugs_with_devices = HashSet::new();
	let pre_anda_present = fda_ctx.is_some_and(|ctx| {
		ctx.studies
			.iter()
			.any(|study| has_text(study.fda_pre_anda_number_occurred.as_deref()))
	});
	let ind_number_present = fda_ctx.is_some_and(|ctx| {
		ctx.studies
			.iter()
			.any(|study| has_text(study.fda_ind_number_occurred.as_deref()))
	});

	for (drug_idx, drug) in validation_ctx.drugs.iter().enumerate() {
		let (devices, device_codes) = list_fda_devices(ctx, mm, drug.id).await?;
		if !devices.is_empty() {
			drugs_with_devices.insert(drug.id);
		}
		let malfunction_this_drug = devices
			.iter()
			.any(|device| device.malfunction == Some(true));
		if drug_idx == 0 {
			first_product_malfunction = malfunction_this_drug;
		}
		for (device_idx, device) in devices.iter().enumerate() {
			if !device_rules_apply {
				continue;
			}
			has_device = true;
			let malfunction = device.malfunction == Some(true);
			fda_g_k_12_r_4_6(
				drug_idx,
				device_idx,
				[
					device.device_brand_name.as_deref(),
					device.common_device_name.as_deref(),
					device.device_product_code.as_deref(),
				],
				(postmarket && combination_true) || malfunction,
				issues,
			);
			if !malfunction {
				continue;
			}
			fda_device_code_rules(
				drug_idx,
				device_idx,
				device,
				&device_codes,
				local_criteria,
				issues,
			);
		}
		if malfunction_this_drug {
			has_malfunction = true;
			if drug.drug_characterization.trim() == "1" {
				has_malfunction_suspect = true;
			}
		}
		fda_g_k_10a(
			drug_idx,
			drug.fda_additional_info_coded.as_deref(),
			drug.fda_additional_info_coded_null_flavor.as_deref(),
			pre_anda_present,
			issues,
		);
		fda_g_k_1_a(
			drug_idx,
			drug.fda_other_characterization.as_deref(),
			combination_true,
			malfunction_this_drug,
			&drug.drug_characterization,
			!vaers,
			issues,
		);
	}
	if postmarket
		&& combination_true
		&& !has_device
		&& !validation_ctx.drugs.is_empty()
	{
		crate::push_business_issue(
			issues,
			"FDA.G.k.12.COLLECTION.REQUIRED",
			"drugs.0.fdaDevices",
			"At least one device is required.",
		);
	}
	fda_d_1_malfunction(validation_ctx, combination_true, has_malfunction, issues);
	if let Some(drug_idx) = validation_ctx
		.drugs
		.iter()
		.position(|drug| drug.drug_characterization.trim() == "1")
	{
		fda_g_k_12(
			drug_idx,
			local_criteria == Some("5"),
			has_malfunction_suspect,
			drugs_with_devices.contains(&validation_ctx.drugs[drug_idx].id),
			issues,
		);
	}
	fda_g_k_1_route(validation_ctx, first_product_malfunction, issues);
	fda_g_k_9(validation_ctx, ind_number_present, issues);
	Ok(())
}

/// MFDS.G.k.2.1.KR.1b.REQUIRED
/// MFDS.G.k.2.1.KR.1b.VOCABULARY
/// MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED
/// MFDS.KR.FOREIGN.WHOMPID.REQUIRED
fn mfds_g_k_2_1_kr_1b(
	idx: usize,
	value: Option<&str>,
	version: Option<&str>,
	receiver: Option<&str>,
	product_code_required: bool,
	domestic_product_code_required: bool,
	foreign_product_code_required: bool,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{idx}.mfdsMpid");
	let present = has_text(value);
	required_when(
		issues,
		"MFDS.G.k.2.1.KR.1b.REQUIRED",
		&path,
		"MFDS requires product code [G.k.2.1.KR.1b] for KR/FR receiver authorities.",
		product_code_required,
		present,
	);
	required_when(
		issues,
		"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		&path,
		"MFDS domestic cases require KR product coding for the drug.",
		domestic_product_code_required,
		present,
	);
	required_when(
		issues,
		"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
		&path,
		"MFDS foreign-use products must provide WHO MPID/KR product coding.",
		foreign_product_code_required,
		present,
	);
	vocabulary(
		issues,
		"MFDS.G.k.2.1.KR.1b.VOCABULARY",
		&path,
		valid_mfds_product(vocabulary_ctx, receiver, version, value),
	);
}

/// MFDS.G.k.2.1.1b.REQUIRED
/// MFDS.G.k.2.1.1a.REQUIRED
/// MFDS.G.k.2.1.2a.REQUIRED
/// MFDS.G.k.2.1.2b.REQUIRED
fn mfds_g_k_2_1_companions(
	idx: usize,
	drug: &DrugInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	for (code, field, missing) in [
		(
			"MFDS.G.k.2.1.1a.REQUIRED",
			"mpidVersion",
			has_text(drug.mpid.as_deref())
				&& !has_text(drug.mpid_version.as_deref()),
		),
		(
			"MFDS.G.k.2.1.1b.REQUIRED",
			"mpid",
			has_text(drug.mpid_version.as_deref())
				&& !has_text(drug.mpid.as_deref()),
		),
		(
			"MFDS.G.k.2.1.2a.REQUIRED",
			"phpidVersion",
			has_text(drug.phpid.as_deref())
				&& !has_text(drug.phpid_version.as_deref()),
		),
		(
			"MFDS.G.k.2.1.2b.REQUIRED",
			"phpid",
			has_text(drug.phpid_version.as_deref())
				&& !has_text(drug.phpid.as_deref()),
		),
	] {
		reject_when(
			issues,
			code,
			&format!("drugs.{idx}.{field}"),
			"unknown",
			code,
			missing,
		);
	}
}

/// MFDS.G.k.2.3.r.2b.REQUIRED
fn mfds_g_k_2_3_r_2b(
	drug_idx: usize,
	idx: usize,
	substance: &DrugActiveSubstance,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_when(
		issues,
		"MFDS.G.k.2.3.r.2b.REQUIRED",
		&format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceTermId"),
		"unknown",
		"MFDS.G.k.2.3.r.2b.REQUIRED",
		has_text(substance.substance_termid_version.as_deref())
			&& !has_text(substance.substance_termid.as_deref()),
	);
}

/// MFDS.G.k.2.1.KR.1a.REQUIRED
fn mfds_g_k_2_1_kr_1a(
	idx: usize,
	value: Option<&str>,
	required: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_when(
		issues,
		"MFDS.G.k.2.1.KR.1a.REQUIRED",
		&format!("drugs.{idx}.mfdsMpidVersion"),
		SECTION,
		"MFDS requires product code version [G.k.2.1.KR.1a] for FR when product code is provided.",
		required && !has_text(value),
	);
}

/// MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED
/// MFDS.G.k.2.3.r.1.KR.1b.REQUIRED
fn mfds_g_k_2_3_r_1_kr_1b(
	drug_idx: usize,
	idx: usize,
	value: Option<&str>,
	version: Option<&str>,
	receiver: Option<&str>,
	domestic_ingredient_code_required: bool,
	substance_code_required: bool,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("drugs.{drug_idx}.activeSubstances.{idx}.mfdsId");
	reject_when(
		issues,
		"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
		&path,
		SECTION,
		"MFDS domestic cases should provide KR ingredient coding for each active substance.",
		domestic_ingredient_code_required && !has_text(value),
	);
	reject_when(
		issues,
		"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
		&path,
		SECTION,
		"MFDS requires substance code [G.k.2.3.r.1.KR.1b] for KR/FR when product code is not provided.",
		substance_code_required && !has_text(value),
	);
	vocabulary(
		issues,
		"MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY",
		&path,
		valid_mfds_substance(vocabulary_ctx, receiver, version, value),
	);
}

/// MFDS.G.k.2.3.r.1.KR.1a.REQUIRED
fn mfds_g_k_2_3_r_1_kr_1a(
	drug_idx: usize,
	idx: usize,
	value: Option<&str>,
	required: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_when(
		issues,
		"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
		&format!("drugs.{drug_idx}.activeSubstances.{idx}.mfdsVersion"),
		SECTION,
		"MFDS requires substance code version [G.k.2.3.r.1.KR.1a] for FR when substance code is provided.",
		required && !has_text(value),
	);
}

/// MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED
#[allow(clippy::too_many_arguments)]
fn mfds_g_k_9_i_2_r_2_kr_1(
	drug_idx: usize,
	idx: usize,
	value: Option<&str>,
	required: bool,
	receiver_is_ct_or_cu: bool,
	receiver_is_kr: bool,
	receiver_is_fr: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"drugs.{drug_idx}.drugReactionAssessments.{idx}.methodOfAssessmentKr1"
	);
	required_when(
		issues,
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		&path,
		"MFDS requires KR method of assessment when source of assessment is present.",
		required,
		has_text(value),
	);
	let invalid = value.map(str::trim).is_some_and(|code| {
		!matches!(code, "1" | "2")
			|| if receiver_is_ct_or_cu {
				code != "2"
			} else if receiver_is_kr {
				code != "1"
			} else {
				receiver_is_fr
			}
	});
	reject_when(
		issues,
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		&path,
		SECTION,
		"MFDS requires KR method of assessment when source of assessment is present.",
		invalid,
	);
}

/// MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED
fn mfds_g_k_9_i_2_r_3_kr_1(
	drug_idx: usize,
	idx: usize,
	value: Option<&str>,
	null_flavor: Option<&str>,
	method: Option<&str>,
	required: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"drugs.{drug_idx}.drugReactionAssessments.{idx}.resultOfAssessmentKr1"
	);
	reject_when(
		issues,
		"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
		&path,
		SECTION,
		"MFDS requires WHO-UMC result when source is present and method is WHO-UMC (1).",
		required && !has_text(value) && null_flavor.is_none(),
	);
	let invalid = method.map(str::trim) == Some("1")
		&& value.map(str::trim).is_some_and(|code| {
			!code.is_empty() && !matches!(code, "1" | "2" | "3" | "4" | "5" | "6")
		});
	reject_when(
		issues,
		"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
		&path,
		SECTION,
		"MFDS requires WHO-UMC result when source is present and method is WHO-UMC (1).",
		invalid,
	);
}

fn mfds_kr1_result_required(
	receiver_is_kr: bool,
	has_source: bool,
	method_is_who_umc: bool,
) -> bool {
	receiver_is_kr && has_source && method_is_who_umc
}

/// MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED
fn mfds_g_k_9_i_2_r_3_kr_2(
	drug_idx: usize,
	idx: usize,
	value: Option<&str>,
	required: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_when(
		issues,
		"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
		&format!(
			"drugs.{drug_idx}.drugReactionAssessments.{idx}.resultOfAssessmentKr2"
		),
		SECTION,
		"MFDS requires KRCT result when source is present, method is KRCT (2), and report is clinical (CT/CU).",
		required && !has_text(value),
	);
}

pub(crate) fn collect_mfds_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|r| r.report_type.as_deref())
		== Some("2");
	let msg_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|h| h.message_receiver_identifier.as_str());
	let receiver_is_kr = is_mfds_domestic_receiver(msg_receiver);
	let receiver_is_fr = is_mfds_foreign_postmarket_receiver(msg_receiver);
	let vocabulary_receiver = if receiver_is_kr {
		Some("KR")
	} else if receiver_is_fr {
		Some("FR")
	} else {
		None
	};
	let receiver_is_ct_or_cu = is_mfds_clinical_trial_receiver(msg_receiver)
		|| is_mfds_compassionate_use_receiver(msg_receiver);

	let mut domestic_drug_ids = HashSet::new();
	let mut drug_index_by_id = HashMap::new();
	let mut drug_has_mfds_mpid_by_id = HashMap::new();

	for (idx, drug) in validation_ctx.drugs.iter().enumerate() {
		drug_index_by_id.insert(drug.id, idx);
		let has_mfds_mpid = has_text(drug.mfds_mpid.as_deref());
		drug_has_mfds_mpid_by_id.insert(drug.id, has_mfds_mpid);
		let country = drug.obtain_drug_country.as_deref().map(str::trim);
		let is_domestic_kr = matches!(country, Some("KR"));
		let is_foreign_non_kr =
			matches!(country, Some(other) if !other.is_empty() && other != "KR");
		if is_domestic_kr {
			domestic_drug_ids.insert(drug.id);
		}
		mfds_g_k_2_1_kr_1b(
			idx,
			drug.mfds_mpid.as_deref(),
			drug.mfds_mpid_version.as_deref(),
			vocabulary_receiver,
			receiver_is_kr || receiver_is_fr,
			is_domestic_kr,
			is_foreign_non_kr,
			&validation_ctx.vocabulary,
			issues,
		);
		mfds_g_k_2_1_kr_1a(
			idx,
			drug.mfds_mpid_version.as_deref(),
			receiver_is_fr && has_mfds_mpid,
			issues,
		);
		mfds_g_k_2_1_companions(idx, drug, issues);
		// MFDS G.k.8: required for clinical-trial and compassionate-use reports.
		if receiver_is_ct_or_cu && !has_text(drug.action_taken.as_deref()) {
			crate::push_business_issue(
				issues,
				"MFDS.G.k.8.REQUIRED",
				format!("drugs.{idx}.actionTaken"),
				"Action Taken with Drug is required for CT and CU reports.",
			);
		}
	}

	let mut mfds_substance_indices = HashMap::new();
	for substance in &mfds_ctx.active_substances {
		let Some(drug_index) = drug_index_by_id.get(&substance.drug_id).copied()
		else {
			continue;
		};
		let substance_index =
			mfds_substance_indices.entry(substance.drug_id).or_insert(0);
		let child_index = *substance_index;
		*substance_index += 1;
		let drug_has_mfds_mpid = drug_has_mfds_mpid_by_id
			.get(&substance.drug_id)
			.copied()
			.unwrap_or(false);
		mfds_g_k_2_3_r_1_kr_1b(
			drug_index,
			child_index,
			substance.mfds_id.as_deref(),
			substance.mfds_version.as_deref(),
			vocabulary_receiver,
			domestic_drug_ids.contains(&substance.drug_id),
			(receiver_is_kr || receiver_is_fr) && !drug_has_mfds_mpid,
			&validation_ctx.vocabulary,
			issues,
		);
		mfds_g_k_2_3_r_1_kr_1a(
			drug_index,
			child_index,
			substance.mfds_version.as_deref(),
			receiver_is_fr && has_text(substance.mfds_id.as_deref()),
			issues,
		);
		mfds_g_k_2_3_r_2b(drug_index, child_index, substance, issues);
	}
	if receiver_is_kr || receiver_is_fr {
		for (drug_idx, drug) in validation_ctx.drugs.iter().enumerate() {
			if !has_text(drug.mfds_mpid.as_deref())
				&& !mfds_ctx.active_substances.iter().any(|substance| {
					substance.drug_id == drug.id
						&& has_text(substance.mfds_id.as_deref())
				}) {
				if !mfds_ctx
					.active_substances
					.iter()
					.any(|row| row.drug_id == drug.id)
				{
					crate::push_business_issue(
						issues,
						"ICH.G.k.2.3.r.REQUIRED",
						format!("drugs.{drug_idx}.activeSubstances"),
						"At least one active substance is required.",
					);
					continue;
				}
				crate::push_business_issue(
					issues,
					"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
					format!("drugs.{drug_idx}.activeSubstances.0.mfdsId"),
					"An MFDS ingredient code is required when the MFDS product code is unavailable.",
				);
			}
		}
	}

	let relatedness_bases = assessment_row_offsets(validation_ctx);
	let mut mfds_relatedness_indices = HashMap::new();
	for r in &mfds_ctx.relatedness {
		let Some(drug_index) = drug_index_by_id.get(&r.drug_id).copied() else {
			continue;
		};
		let Some(base) = relatedness_bases.get(&r.drug_reaction_assessment_id)
		else {
			continue;
		};
		let assessment_index = mfds_relatedness_indices
			.entry(r.drug_reaction_assessment_id)
			.or_insert(0);
		let child_index = base + *assessment_index;
		*assessment_index += 1;
		let has_source = has_text(r.source_of_assessment.as_deref());
		let method_code = r.method_of_assessment_kr1.as_deref().map(str::trim);
		let method_is_who_umc = method_code == Some("1");
		let method_is_krct = method_code == Some("2");
		let method_required_context = has_source || receiver_is_ct_or_cu;
		let kr2_required_context = has_source
			&& method_is_krct
			&& (report_type_is_study || receiver_is_ct_or_cu);
		mfds_g_k_9_i_2_r_2_kr_1(
			drug_index,
			child_index,
			r.method_of_assessment_kr1.as_deref(),
			method_required_context,
			receiver_is_ct_or_cu,
			receiver_is_kr,
			receiver_is_fr,
			issues,
		);
		mfds_g_k_9_i_2_r_3_kr_1(
			drug_index,
			child_index,
			r.result_of_assessment_kr1.as_deref(),
			r.result_of_assessment_kr1_null_flavor.as_deref(),
			r.method_of_assessment_kr1.as_deref(),
			mfds_kr1_result_required(receiver_is_kr, has_source, method_is_who_umc),
			issues,
		);
		mfds_g_k_9_i_2_r_3_kr_2(
			drug_index,
			child_index,
			r.result_of_assessment_kr2.as_deref(),
			kr2_required_context,
			issues,
		);
	}
}

#[cfg(test)]
#[path = "g/field_rule_tests.rs"]
mod field_rule_tests;

#[cfg(test)]
#[path = "g/golden_g_required_tests.rs"]
mod golden_g_required_tests;

#[cfg(test)]
#[path = "g/rule_tests.rs"]
mod rule_tests;
