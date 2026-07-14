use super::rule_table::{
	eval_catalog_values, eval_companions, eval_grandchild_length, eval_indexed,
	eval_indexed_constraints, eval_indexed_derived_length,
	eval_indexed_future_dates, eval_indexed_length,
	eval_indexed_vocabulary_variants, eval_nested_constraints,
	eval_nested_derived_length, eval_nested_length, eval_nested_meddra,
	CatalogValueRule, CompanionRule, DateValues, GrandchildLengthRule,
	IndexedConstraintRule, IndexedDerivedLengthRule, IndexedFutureDateRule,
	IndexedLengthRule, IndexedRule, IndexedVocabularyVariantRule,
	NestedConstraintRule, NestedDerivedLengthRule, NestedLengthRule,
	NestedMeddraRule, RuleValue,
};
use crate::allowed_value::{true_marker_value, ConstraintValue};
use crate::{
	has_text, is_mfds_clinical_trial_receiver, is_mfds_compassionate_use_receiver,
	is_mfds_domestic_receiver, is_mfds_foreign_postmarket_receiver,
	list_drug_characteristics, push_issue_by_code, FdaValidationContext,
	MfdsValidationContext, RegulatoryAuthority, RuleFacts, ValidationContext,
	ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::drug::{
	derive_fda_device_characteristics, parse_drug_additional_info_codes_json,
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use lib_core::model::{ModelManager, Result};
use sqlx::types::Decimal;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

const G_MFDS_PRODUCT_VOCABULARY_RULES: &[IndexedVocabularyVariantRule<
	DrugInformation,
>] = &[IndexedVocabularyVariantRule {
	code: "MFDS.G.k.2.1.KR.1b.VOCABULARY",
	path: |idx| format!("drugs.{idx}.mfdsMpid"),
	value: |item| item.mfds_mpid.as_deref(),
}];

fn normalize_code(raw: Option<&str>) -> String {
	raw.unwrap_or("")
		.trim()
		.to_ascii_uppercase()
		.replace(['.', '_', '-'], "")
}

fn characteristic_code_matches(raw: Option<&str>, target: &str) -> bool {
	let raw = normalize_code(raw);
	let target = normalize_code(Some(target));
	if raw == target {
		return true;
	}
	match target.as_str() {
		"FDAGK12R1" => raw == "C54026",
		"FDAGK12R2R" => raw == "C54592",
		"FDAGK12R3" => raw == "C54451" || raw == "FDAGK12R3R",
		"FDAGK12R8" => raw == "C54595",
		"FDAGK12R11" => raw == "C54594" || raw == "FDAGK12R11R",
		_ => false,
	}
}

fn is_truthy_characteristic(ch: &DrugDeviceCharacteristic) -> bool {
	let code = ch.value_code.as_deref().map(str::trim).unwrap_or("");
	let value = ch.value_value.as_deref().map(str::trim).unwrap_or("");
	matches!(code, "1" | "true" | "TRUE" | "True")
		|| matches!(value, "1" | "true" | "TRUE" | "True")
}

fn is_code_one_characteristic(ch: &DrugDeviceCharacteristic) -> bool {
	let code = ch.value_code.as_deref().map(str::trim).unwrap_or("");
	let value = ch.value_value.as_deref().map(str::trim).unwrap_or("");
	code == "1" || value == "1"
}

fn has_characteristic_value(
	chars: &[DrugDeviceCharacteristic],
	target: &str,
) -> bool {
	chars.iter().any(|ch| {
		characteristic_code_matches(ch.code.as_deref(), target)
			&& (has_text(ch.value_value.as_deref())
				|| has_text(ch.value_code.as_deref())
				|| has_text(ch.value_display_name.as_deref()))
	})
}

fn decimal_text(value: Option<Decimal>) -> Option<String> {
	value.map(|value| value.to_string())
}

fn i32_text(value: Option<i32>) -> Option<String> {
	value.map(|value| value.to_string())
}

struct MfdsDrugRuleView {
	index: usize,
	mpid: Option<String>,
	mpid_version: Option<String>,
	facts: RuleFacts,
}

const G_MFDS_DRUG_CATALOG_VALUE_RULES: &[CatalogValueRule<MfdsDrugRuleView>] = &[
	CatalogValueRule {
		code: "MFDS.G.k.2.1.KR.1b.REQUIRED",
		path: |item| format!("drugs.{}.mfdsMpid", item.index),
		value: |item| RuleValue::borrowed(item.mpid.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "MFDS.G.k.2.1.KR.1a.REQUIRED",
		path: |item| format!("drugs.{}.mfdsMpidVersion", item.index),
		value: |item| RuleValue::borrowed(item.mpid_version.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		path: |item| format!("drugs.{}.mfdsMpid", item.index),
		value: |item| RuleValue::borrowed(item.mpid.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
		path: |item| format!("drugs.{}.mfdsMpid", item.index),
		value: |item| RuleValue::borrowed(item.mpid.as_deref(), None),
		facts: |item| item.facts,
	},
];

struct MfdsSubstanceRuleView {
	drug_index: usize,
	substance_index: usize,
	id: Option<String>,
	version: Option<String>,
	facts: RuleFacts,
}

const G_MFDS_SUBSTANCE_CATALOG_VALUE_RULES: &[CatalogValueRule<
	MfdsSubstanceRuleView,
>] = &[
	CatalogValueRule {
		code: "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
		path: |item| {
			format!(
				"drugs.{}.activeSubstances.{}.mfdsId",
				item.drug_index, item.substance_index
			)
		},
		value: |item| RuleValue::borrowed(item.id.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
		path: |item| {
			format!(
				"drugs.{}.activeSubstances.{}.mfdsId",
				item.drug_index, item.substance_index
			)
		},
		value: |item| RuleValue::borrowed(item.id.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
		path: |item| {
			format!(
				"drugs.{}.activeSubstances.{}.mfdsVersion",
				item.drug_index, item.substance_index
			)
		},
		value: |item| RuleValue::borrowed(item.version.as_deref(), None),
		facts: |item| item.facts,
	},
];

struct MfdsRelatednessRuleView {
	drug_index: usize,
	assessment_index: usize,
	source: Option<String>,
	method: Option<String>,
	result_kr1: Option<String>,
	result_kr2: Option<String>,
	facts: RuleFacts,
}

impl MfdsRelatednessRuleView {
	fn path(&self, field: &str) -> String {
		format!(
			"drugs.{}.drugReactionAssessments.{}.{}",
			self.drug_index, self.assessment_index, field
		)
	}
}

const G_MFDS_RELATEDNESS_CATALOG_VALUE_RULES: &[CatalogValueRule<
	MfdsRelatednessRuleView,
>] = &[
	CatalogValueRule {
		code: "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		path: |item| item.path("methodOfAssessment"),
		value: |item| RuleValue::borrowed(item.method.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
		path: |item| item.path("resultOfAssessment"),
		value: |item| RuleValue::borrowed(item.result_kr1.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
		path: |item| item.path("resultOfAssessmentKr2"),
		value: |item| RuleValue::borrowed(item.result_kr2.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "MFDS.G.k.9.i.2.r.1.REQUIRED",
		path: |item| item.path("sourceOfAssessment"),
		value: |item| RuleValue::borrowed(item.source.as_deref(), None),
		facts: |item| item.facts,
	},
];

fn resolve_drug_child_indices(
	drug_indices: &HashMap<sqlx::types::Uuid, usize>,
	drug_id: sqlx::types::Uuid,
	sequence_number: i32,
) -> Option<(usize, usize)> {
	let drug_index = drug_indices.get(&drug_id).copied()?;
	let child_index = sequence_number
		.checked_sub(1)
		.and_then(|value| usize::try_from(value).ok())?;
	Some((drug_index, child_index))
}

fn sequence_idx(sequence_number: i32, fallback: usize) -> usize {
	sequence_number
		.checked_sub(1)
		.and_then(|value| usize::try_from(value).ok())
		.unwrap_or(fallback)
}

fn longest_additional_info_code(drug: &DrugInformation) -> Option<String> {
	additional_info_codes(drug)
		.into_iter()
		.max_by_key(|value| value.chars().count())
}

fn additional_info_codes(drug: &DrugInformation) -> Vec<String> {
	parse_drug_additional_info_codes_json(
		drug.drug_additional_info_codes_json.as_ref(),
	)
	.into_iter()
	.filter_map(|entry| entry.value_code)
	.collect()
}

const G_DRUG_VALUE_RULES: &[IndexedRule<DrugInformation>] = &[
	IndexedRule {
		code: "ICH.G.k.1.REQUIRED",
		path: |idx| format!("drugs.{idx}.drugCharacterization"),
		value: |drug| {
			RuleValue::borrowed(Some(drug.drug_characterization.as_str()), None)
		},
		facts: |_| RuleFacts::default(),
	},
	IndexedRule {
		code: "ICH.G.k.2.2.REQUIRED",
		path: |idx| format!("drugs.{idx}.medicinalProduct"),
		value: |drug| {
			RuleValue::borrowed(Some(drug.medicinal_product.as_str()), None)
		},
		facts: |_| RuleFacts::default(),
	},
];

const G_DRUG_LENGTH_RULES: &[IndexedLengthRule<DrugInformation>] = &[
	IndexedLengthRule {
		code: "ICH.G.k.1.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.drugCharacterization"),
		value: |drug| Some(drug.drug_characterization.as_str()),
	},
	IndexedLengthRule {
		code: "ICH.G.k.2.1.1a.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.mpidVersion"),
		value: |drug| drug.mpid_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.2.1.1b.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.mpid"),
		value: |drug| drug.mpid.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.2.1.2a.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.phpidVersion"),
		value: |drug| drug.phpid_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.2.1.2b.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.phpid"),
		value: |drug| drug.phpid.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.2.2.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.medicinalProduct"),
		value: |drug| Some(drug.medicinal_product.as_str()),
	},
	IndexedLengthRule {
		code: "ICH.G.k.2.4.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.obtainDrugCountry"),
		value: |drug| drug.obtain_drug_country.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.3.1.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.drugAuthorizationNumber"),
		value: |drug| drug.drug_authorization_number.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.3.2.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.drugAuthorizationCountry"),
		value: |drug| drug.manufacturer_country.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.3.3.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.manufacturerName"),
		value: |drug| drug.manufacturer_name.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.5b.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.cumulativeDoseFirstReactionUnit"),
		value: |drug| drug.cumulative_dose_first_reaction_unit.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.6b.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.gestationPeriodExposureUnit"),
		value: |drug| drug.gestation_period_exposure_unit.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.8.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.actionTaken"),
		value: |drug| drug.action_taken.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.G.k.11.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.drugAdditionalInformation"),
		value: |drug| drug.drug_additional_information.as_deref(),
	},
];

const G_DRUG_DERIVED_LENGTH_RULES: &[IndexedDerivedLengthRule<DrugInformation>] = &[
	IndexedDerivedLengthRule {
		code: "ICH.G.k.5a.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.cumulativeDoseFirstReactionValue"),
		value: |drug| decimal_text(drug.cumulative_dose_first_reaction_value),
	},
	IndexedDerivedLengthRule {
		code: "ICH.G.k.6a.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.gestationPeriodExposureValue"),
		value: |drug| decimal_text(drug.gestation_period_exposure_value),
	},
	IndexedDerivedLengthRule {
		code: "ICH.G.k.10.r.LENGTH.MAX",
		path: |idx| format!("drugs.{idx}.drugAdditionalInformationCodes"),
		value: longest_additional_info_code,
	},
];

const G_DRUG_CONSTRAINT_RULES: &[IndexedConstraintRule<DrugInformation>] = &[
	IndexedConstraintRule {
		code: "ICH.G.k.2.1.1b.ALLOWED.VALUE",
		path: |idx| format!("drugs.{idx}.mpid"),
		value: |drug| ConstraintValue::Text(drug.mpid.as_deref().map(Cow::Borrowed)),
	},
	IndexedConstraintRule {
		code: "ICH.G.k.2.1.2b.ALLOWED.VALUE",
		path: |idx| format!("drugs.{idx}.phpid"),
		value: |drug| {
			ConstraintValue::Text(drug.phpid.as_deref().map(Cow::Borrowed))
		},
	},
	IndexedConstraintRule {
		code: "ICH.G.k.1.ALLOWED.VALUE",
		path: |idx| format!("drugs.{idx}.drugCharacterization"),
		value: |drug| {
			ConstraintValue::Text(Some(Cow::Borrowed(
				drug.drug_characterization.as_str(),
			)))
		},
	},
	IndexedConstraintRule {
		code: "ICH.G.k.8.ALLOWED.VALUE",
		path: |idx| format!("drugs.{idx}.actionTaken"),
		value: |drug| {
			ConstraintValue::Text(drug.action_taken.as_deref().map(Cow::Borrowed))
		},
	},
	IndexedConstraintRule {
		code: "ICH.G.k.2.4.VOCABULARY",
		path: |idx| format!("drugs.{idx}.obtainDrugCountry"),
		value: |drug| {
			ConstraintValue::Text(
				drug.obtain_drug_country.as_deref().map(Cow::Borrowed),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.G.k.3.2.VOCABULARY",
		path: |idx| format!("drugs.{idx}.drugAuthorizationCountry"),
		value: |drug| {
			ConstraintValue::Text(
				drug.manufacturer_country.as_deref().map(Cow::Borrowed),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.G.k.10.r.ALLOWED.VALUE",
		path: |idx| format!("drugs.{idx}.drugAdditionalInformationCodes"),
		value: |drug| {
			ConstraintValue::Texts(
				additional_info_codes(drug)
					.into_iter()
					.map(Cow::Owned)
					.collect(),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.G.k.2.5.ALLOWED.VALUE",
		path: |idx| format!("drugs.{idx}.investigationalProductBlinded"),
		value: |drug| true_marker_value(drug.investigational_product_blinded, None),
	},
];

const G_DRUG_COMPANION_RULES: &[CompanionRule<DrugInformation>] = &[
	CompanionRule {
		code: "ICH.G.k.5a.REQUIRED",
		path: |idx| format!("drugs.{idx}.cumulativeDoseFirstReactionValue"),
		trigger: |drug| {
			has_text(drug.cumulative_dose_first_reaction_unit.as_deref())
		},
		required: |drug| drug.cumulative_dose_first_reaction_value.is_some(),
	},
	CompanionRule {
		code: "ICH.G.k.5b.REQUIRED",
		path: |idx| format!("drugs.{idx}.cumulativeDoseFirstReactionUnit"),
		trigger: |drug| drug.cumulative_dose_first_reaction_value.is_some(),
		required: |drug| {
			has_text(drug.cumulative_dose_first_reaction_unit.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.G.k.6a.REQUIRED",
		path: |idx| format!("drugs.{idx}.gestationPeriodExposureValue"),
		trigger: |drug| has_text(drug.gestation_period_exposure_unit.as_deref()),
		required: |drug| drug.gestation_period_exposure_value.is_some(),
	},
	CompanionRule {
		code: "ICH.G.k.6b.REQUIRED",
		path: |idx| format!("drugs.{idx}.gestationPeriodExposureUnit"),
		trigger: |drug| drug.gestation_period_exposure_value.is_some(),
		required: |drug| has_text(drug.gestation_period_exposure_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.3.2.REQUIRED",
		path: |idx| format!("drugs.{idx}.drugAuthorizationCountry"),
		trigger: |drug| has_text(drug.drug_authorization_number.as_deref()),
		required: |drug| has_text(drug.manufacturer_country.as_deref()),
	},
];

const G_ACTIVE_SUBSTANCE_LENGTH_RULES: &[NestedLengthRule<DrugActiveSubstance>] = &[
	NestedLengthRule {
		code: "ICH.G.k.2.3.r.1.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceName")
		},
		value: |substance| substance.substance_name.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.2.3.r.2a.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceTermIdVersion")
		},
		value: |substance| substance.substance_termid_version.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.2.3.r.2b.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceTermId")
		},
		value: |substance| substance.substance_termid.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.2.3.r.3b.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.activeSubstances.{idx}.strengthUnit")
		},
		value: |substance| substance.strength_unit.as_deref(),
	},
];

const G_ACTIVE_SUBSTANCE_CONSTRAINT_RULES: &[NestedConstraintRule<
	DrugActiveSubstance,
>] = &[
	NestedConstraintRule {
		code: "ICH.G.k.2.3.r.2b.ALLOWED.VALUE",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.activeSubstances.{idx}.substanceTermId")
		},
		value: |substance| {
			ConstraintValue::Text(
				substance.substance_termid.as_deref().map(Cow::Borrowed),
			)
		},
	},
	NestedConstraintRule {
		code: "ICH.G.k.2.3.r.3b.ALLOWED.VALUE",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.activeSubstances.{idx}.strengthUnit")
		},
		value: |substance| {
			ConstraintValue::Text(
				substance.strength_unit.as_deref().map(Cow::Borrowed),
			)
		},
	},
];

const G_ACTIVE_SUBSTANCE_DERIVED_LENGTH_RULES: &[NestedDerivedLengthRule<
	DrugActiveSubstance,
>] = &[NestedDerivedLengthRule {
	code: "ICH.G.k.2.3.r.3a.LENGTH.MAX",
	path: |drug_idx, idx| {
		format!("drugs.{drug_idx}.activeSubstances.{idx}.strengthValue")
	},
	value: |substance| decimal_text(substance.strength_value),
}];

const G_ACTIVE_SUBSTANCE_COMPANION_RULES: &[CompanionRule<DrugActiveSubstance>] = &[
	CompanionRule {
		code: "ICH.G.k.2.3.r.1.REQUIRED",
		path: |idx| format!("drugs.0.activeSubstances.{idx}.substanceName"),
		trigger: |_| true,
		required: |substance| {
			has_text(substance.substance_termid.as_deref())
				|| has_text(substance.substance_name.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.G.k.2.3.r.2a.REQUIRED",
		path: |idx| format!("drugs.0.activeSubstances.{idx}.substanceTermIdVersion"),
		trigger: |substance| has_text(substance.substance_termid.as_deref()),
		required: |substance| {
			has_text(substance.substance_termid_version.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.G.k.2.3.r.3b.REQUIRED",
		path: |idx| format!("drugs.0.activeSubstances.{idx}.strengthUnit"),
		trigger: |substance| substance.strength_value.is_some(),
		required: |substance| has_text(substance.strength_unit.as_deref()),
	},
];

const G_DOSAGE_LENGTH_RULES: &[NestedLengthRule<DosageInformation>] = &[
	NestedLengthRule {
		code: "ICH.G.k.4.r.1b.LENGTH.MAX",
		path: |drug_idx, idx| format!("drugs.{drug_idx}.dosages.{idx}.doseUnit"),
		value: |dosage| dosage.dose_unit.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.3.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.frequencyUnit")
		},
		value: |dosage| dosage.frequency_unit.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.6b.LENGTH.MAX",
		path: |drug_idx, idx| format!("drugs.{drug_idx}.dosages.{idx}.durationUnit"),
		value: |dosage| dosage.duration_unit.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.7.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.batchLotNumber")
		},
		value: |dosage| dosage.batch_lot_number.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.8.LENGTH.MAX",
		path: |drug_idx, idx| format!("drugs.{drug_idx}.dosages.{idx}.dosageText"),
		value: |dosage| dosage.dosage_text.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.9.1.LENGTH.MAX",
		path: |drug_idx, idx| format!("drugs.{drug_idx}.dosages.{idx}.doseForm"),
		value: |dosage| dosage.dose_form.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.9.2a.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.doseFormTermIdVersion")
		},
		value: |dosage| dosage.dose_form_termid_version.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.9.2b.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.doseFormTermId")
		},
		value: |dosage| dosage.dose_form_termid.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.10.1.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.routeOfAdministration")
		},
		value: |dosage| dosage.route_of_administration.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.10.2a.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.routeTermIdVersion")
		},
		value: |dosage| dosage.route_termid_version.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.10.2b.LENGTH.MAX",
		path: |drug_idx, idx| format!("drugs.{drug_idx}.dosages.{idx}.routeTermId"),
		value: |dosage| dosage.route_termid.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.11.1.LENGTH.MAX",
		path: |drug_idx, idx| format!("drugs.{drug_idx}.dosages.{idx}.parentRoute"),
		value: |dosage| dosage.parent_route.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.11.2a.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.parentRouteTermIdVersion")
		},
		value: |dosage| dosage.parent_route_termid_version.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.4.r.11.2b.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.parentRouteTermId")
		},
		value: |dosage| dosage.parent_route_termid.as_deref(),
	},
];

const G_DOSAGE_DERIVED_LENGTH_RULES: &[NestedDerivedLengthRule<
	DosageInformation,
>] = &[
	NestedDerivedLengthRule {
		code: "ICH.G.k.4.r.1a.LENGTH.MAX",
		path: |drug_idx, idx| format!("drugs.{drug_idx}.dosages.{idx}.doseValue"),
		value: |dosage| decimal_text(dosage.dose_value),
	},
	NestedDerivedLengthRule {
		code: "ICH.G.k.4.r.2.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.numberOfUnits")
		},
		value: |dosage| i32_text(dosage.number_of_units),
	},
	NestedDerivedLengthRule {
		code: "ICH.G.k.4.r.6a.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.dosages.{idx}.durationValue")
		},
		value: |dosage| decimal_text(dosage.duration_value),
	},
];

const G_DOSAGE_COMPANION_RULES: &[CompanionRule<DosageInformation>] = &[
	CompanionRule {
		code: "ICH.G.k.4.r.1b.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.doseUnit"),
		trigger: |dosage| dosage.dose_value.is_some(),
		required: |dosage| has_text(dosage.dose_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.3.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.frequencyUnit"),
		trigger: |dosage| dosage.frequency_value.is_some(),
		required: |dosage| has_text(dosage.frequency_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.6a.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.durationValue"),
		trigger: |dosage| has_text(dosage.duration_unit.as_deref()),
		required: |dosage| dosage.duration_value.is_some(),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.6b.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.durationUnit"),
		trigger: |dosage| dosage.duration_value.is_some(),
		required: |dosage| has_text(dosage.duration_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.9.2a.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.doseFormTermIdVersion"),
		trigger: |dosage| has_text(dosage.dose_form_termid.as_deref()),
		required: |dosage| has_text(dosage.dose_form_termid_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.10.2a.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.routeTermIdVersion"),
		trigger: |dosage| has_text(dosage.route_of_administration.as_deref()),
		required: |dosage| has_text(dosage.route_termid_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.11.2a.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.parentRouteTermIdVersion"),
		trigger: |dosage| has_text(dosage.parent_route_termid.as_deref()),
		required: |dosage| has_text(dosage.parent_route_termid_version.as_deref()),
	},
];

const G_DOSAGE_FUTURE_DATE_RULES: &[IndexedFutureDateRule<DosageInformation>] =
	&[IndexedFutureDateRule {
		code: "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
		path: |idx| format!("drugs.0.dosageInformation.{idx}.dateRange"),
		dates: |dosage| {
			DateValues::Two(
				dosage.first_administration_date,
				dosage.last_administration_date,
			)
		},
	}];

const G_INDICATION_LENGTH_RULES: &[NestedLengthRule<DrugIndication>] = &[
	NestedLengthRule {
		code: "ICH.G.k.7.r.1.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.indications.{idx}.indicationText")
		},
		value: |indication| indication.indication_text.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.7.r.2a.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraVersion")
		},
		value: |indication| indication.indication_meddra_version.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.7.r.2b.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraCode")
		},
		value: |indication| indication.indication_meddra_code.as_deref(),
	},
];

const G_INDICATION_COMPANION_RULES: &[CompanionRule<DrugIndication>] = &[
	CompanionRule {
		code: "ICH.G.k.7.r.2a.REQUIRED",
		path: |idx| format!("drugs.0.indications.{idx}.indicationMeddraVersion"),
		trigger: |indication| has_text(indication.indication_meddra_code.as_deref()),
		required: |indication| {
			has_text(indication.indication_meddra_version.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.G.k.7.r.2b.REQUIRED",
		path: |idx| format!("drugs.0.indications.{idx}.indicationMeddraCode"),
		trigger: |indication| {
			has_text(indication.indication_meddra_version.as_deref())
		},
		required: |indication| {
			has_text(indication.indication_meddra_code.as_deref())
		},
	},
];

const G_INDICATION_MEDDRA_RULES: &[NestedMeddraRule<DrugIndication>] =
	&[NestedMeddraRule {
		version_allowed_code: "ICH.G.k.7.r.2a.ALLOWED.VALUE",
		version_code: "ICH.G.k.7.r.2a.VOCABULARY",
		code_allowed_code: "ICH.G.k.7.r.2b.ALLOWED.VALUE",
		code_code: "ICH.G.k.7.r.2b.VOCABULARY",
		version_path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraVersion")
		},
		code_path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraCode")
		},
		values: |indication| {
			(
				indication.indication_meddra_version.as_deref(),
				indication.indication_meddra_code.as_deref(),
			)
		},
	}];

const G_REACTION_ASSESSMENT_LENGTH_RULES: &[NestedLengthRule<
	DrugReactionAssessment,
>] = &[
	NestedLengthRule {
		code: "ICH.G.k.9.i.3.1b.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!(
				"drugs.{drug_idx}.reactionAssessments.{idx}.administrationStartIntervalUnit"
			)
		},
		value: |assessment| assessment.administration_start_interval_unit.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.9.i.3.2b.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!(
				"drugs.{drug_idx}.reactionAssessments.{idx}.lastDoseIntervalUnit"
			)
		},
		value: |assessment| assessment.last_dose_interval_unit.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.G.k.9.i.4.LENGTH.MAX",
		path: |drug_idx, idx| {
			format!("drugs.{drug_idx}.reactionAssessments.{idx}.reactionRecurred")
		},
		value: |assessment| assessment.reaction_recurred.as_deref(),
	},
];

const G_REACTION_ASSESSMENT_DERIVED_LENGTH_RULES: &[NestedDerivedLengthRule<
	DrugReactionAssessment,
>] =
	&[
		NestedDerivedLengthRule {
			code: "ICH.G.k.9.i.3.1a.LENGTH.MAX",
			path: |drug_idx, idx| {
				format!(
				"drugs.{drug_idx}.reactionAssessments.{idx}.administrationStartIntervalValue"
			)
			},
			value: |assessment| {
				decimal_text(assessment.administration_start_interval_value)
			},
		},
		NestedDerivedLengthRule {
			code: "ICH.G.k.9.i.3.2a.LENGTH.MAX",
			path: |drug_idx, idx| {
				format!("drugs.{drug_idx}.reactionAssessments.{idx}.lastDoseIntervalValue")
			},
			value: |assessment| decimal_text(assessment.last_dose_interval_value),
		},
	];

const G_RELATEDNESS_ASSESSMENT_LENGTH_RULES: &[GrandchildLengthRule<
	RelatednessAssessment,
>] = &[
	GrandchildLengthRule {
		code: "ICH.G.k.9.i.2.r.1.LENGTH.MAX",
		path: |drug_idx, assessment_idx, idx| {
			format!(
				"drugs.{drug_idx}.reactionAssessments.{assessment_idx}.relatednessAssessments.{idx}.sourceOfAssessment"
			)
		},
		value: |relatedness| relatedness.source_of_assessment.as_deref(),
	},
	GrandchildLengthRule {
		code: "ICH.G.k.9.i.2.r.2.LENGTH.MAX",
		path: |drug_idx, assessment_idx, idx| {
			format!(
				"drugs.{drug_idx}.reactionAssessments.{assessment_idx}.relatednessAssessments.{idx}.methodOfAssessment"
			)
		},
		value: |relatedness| relatedness.method_of_assessment.as_deref(),
	},
	GrandchildLengthRule {
		code: "ICH.G.k.9.i.2.r.3.LENGTH.MAX",
		path: |drug_idx, assessment_idx, idx| {
			format!(
				"drugs.{drug_idx}.reactionAssessments.{assessment_idx}.relatednessAssessments.{idx}.resultOfAssessment"
			)
		},
		value: |relatedness| relatedness.result_of_assessment.as_deref(),
	},
];

const G_REACTION_ASSESSMENT_CONSTRAINT_RULES: &[NestedConstraintRule<
	DrugReactionAssessment,
>] = &[NestedConstraintRule {
	code: "ICH.G.k.9.i.4.ALLOWED.VALUE",
	path: |drug_idx, idx| {
		format!("drugs.{drug_idx}.reactionAssessments.{idx}.reactionRecurred")
	},
	value: |assessment| {
		ConstraintValue::Text(
			assessment.reaction_recurred.as_deref().map(Cow::Borrowed),
		)
	},
}];

const G_REACTION_ASSESSMENT_COMPANION_RULES: &[CompanionRule<
	DrugReactionAssessment,
>] =
	&[
		CompanionRule {
			code: "ICH.G.k.9.i.3.1a.REQUIRED",
			path: |idx| {
				format!("drugs.0.reactionAssessments.{idx}.administrationStartIntervalValue")
			},
			trigger: |assessment| {
				has_text(assessment.administration_start_interval_unit.as_deref())
			},
			required: |assessment| {
				assessment.administration_start_interval_value.is_some()
			},
		},
		CompanionRule {
			code: "ICH.G.k.9.i.3.1b.REQUIRED",
			path: |idx| {
				format!("drugs.0.reactionAssessments.{idx}.administrationStartIntervalUnit")
			},
			trigger: |assessment| {
				assessment.administration_start_interval_value.is_some()
			},
			required: |assessment| {
				has_text(assessment.administration_start_interval_unit.as_deref())
			},
		},
		CompanionRule {
			code: "ICH.G.k.9.i.3.2a.REQUIRED",
			path: |idx| {
				format!("drugs.0.reactionAssessments.{idx}.lastDoseIntervalValue")
			},
			trigger: |assessment| {
				has_text(assessment.last_dose_interval_unit.as_deref())
			},
			required: |assessment| assessment.last_dose_interval_value.is_some(),
		},
		CompanionRule {
			code: "ICH.G.k.9.i.3.2b.REQUIRED",
			path: |idx| {
				format!("drugs.0.reactionAssessments.{idx}.lastDoseIntervalUnit")
			},
			trigger: |assessment| assessment.last_dose_interval_value.is_some(),
			required: |assessment| {
				has_text(assessment.last_dose_interval_unit.as_deref())
			},
		},
	];

pub(crate) async fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	mm: &ModelManager,
	ctx: &Ctx,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) -> Result<()> {
	let _ = fda_ctx;
	collect_ich_issues(validation_ctx, issues);
	match authority {
		RegulatoryAuthority::Ich => {}
		RegulatoryAuthority::Fda => {
			collect_fda_issues(ctx, mm, validation_ctx, issues).await?
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
	if validation_ctx.drugs.is_empty() {
		push_issue_by_code(
			issues,
			"ICH.G.k.1.REQUIRED",
			"drugs.0.drugCharacterization",
		);
		push_issue_by_code(
			issues,
			"ICH.G.k.2.2.REQUIRED",
			"drugs.0.medicinalProduct",
		);
	}

	eval_indexed(issues, &validation_ctx.drugs, G_DRUG_VALUE_RULES);
	eval_indexed_length(issues, &validation_ctx.drugs, G_DRUG_LENGTH_RULES);
	eval_indexed_derived_length(
		issues,
		&validation_ctx.drugs,
		G_DRUG_DERIVED_LENGTH_RULES,
	);
	eval_indexed_constraints(
		issues,
		&validation_ctx.drugs,
		G_DRUG_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_companions(issues, &validation_ctx.drugs, G_DRUG_COMPANION_RULES);
	eval_nested_length(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.active_substances,
		|drug| drug.id,
		|substance| substance.drug_id,
		|substance, fallback| sequence_idx(substance.sequence_number, fallback),
		G_ACTIVE_SUBSTANCE_LENGTH_RULES,
	);
	eval_nested_constraints(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.active_substances,
		|drug| drug.id,
		|substance| substance.drug_id,
		|substance, fallback| sequence_idx(substance.sequence_number, fallback),
		G_ACTIVE_SUBSTANCE_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_nested_derived_length(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.active_substances,
		|drug| drug.id,
		|substance| substance.drug_id,
		|substance, fallback| sequence_idx(substance.sequence_number, fallback),
		G_ACTIVE_SUBSTANCE_DERIVED_LENGTH_RULES,
	);
	eval_companions(
		issues,
		&validation_ctx.active_substances,
		G_ACTIVE_SUBSTANCE_COMPANION_RULES,
	);

	eval_nested_length(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.dosages,
		|drug| drug.id,
		|dosage| dosage.drug_id,
		|dosage, fallback| sequence_idx(dosage.sequence_number, fallback),
		G_DOSAGE_LENGTH_RULES,
	);
	eval_nested_derived_length(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.dosages,
		|drug| drug.id,
		|dosage| dosage.drug_id,
		|dosage, fallback| sequence_idx(dosage.sequence_number, fallback),
		G_DOSAGE_DERIVED_LENGTH_RULES,
	);
	eval_companions(issues, &validation_ctx.dosages, G_DOSAGE_COMPANION_RULES);
	eval_indexed_future_dates(
		issues,
		&validation_ctx.dosages,
		G_DOSAGE_FUTURE_DATE_RULES,
	);

	eval_nested_length(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.indications,
		|drug| drug.id,
		|indication| indication.drug_id,
		|indication, fallback| sequence_idx(indication.sequence_number, fallback),
		G_INDICATION_LENGTH_RULES,
	);
	eval_companions(
		issues,
		&validation_ctx.indications,
		G_INDICATION_COMPANION_RULES,
	);
	eval_nested_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.drugs,
		&validation_ctx.indications,
		|drug| drug.id,
		|indication| indication.drug_id,
		|indication, fallback| sequence_idx(indication.sequence_number, fallback),
		G_INDICATION_MEDDRA_RULES,
	);
	eval_nested_length(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.drug_reaction_assessments,
		|drug| drug.id,
		|assessment| assessment.drug_id,
		|_, fallback| fallback,
		G_REACTION_ASSESSMENT_LENGTH_RULES,
	);
	eval_nested_derived_length(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.drug_reaction_assessments,
		|drug| drug.id,
		|assessment| assessment.drug_id,
		|_, fallback| fallback,
		G_REACTION_ASSESSMENT_DERIVED_LENGTH_RULES,
	);
	eval_nested_constraints(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.drug_reaction_assessments,
		|drug| drug.id,
		|assessment| assessment.drug_id,
		|_, fallback| fallback,
		G_REACTION_ASSESSMENT_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_grandchild_length(
		issues,
		&validation_ctx.drugs,
		&validation_ctx.drug_reaction_assessments,
		&validation_ctx.relatedness_assessments,
		|drug| drug.id,
		|assessment| assessment.id,
		|assessment| assessment.drug_id,
		|relatedness| relatedness.drug_reaction_assessment_id,
		|_, fallback| fallback,
		|relatedness, fallback| sequence_idx(relatedness.sequence_number, fallback),
		G_RELATEDNESS_ASSESSMENT_LENGTH_RULES,
	);
	eval_companions(
		issues,
		&validation_ctx.drug_reaction_assessments,
		G_REACTION_ASSESSMENT_COMPANION_RULES,
	);
}

pub(crate) async fn collect_fda_issues(
	ctx: &Ctx,
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
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
		== Some("1");

	let mut has_malfunction_any = false;
	let mut has_malfunction_suspect = false;
	let mut has_gk12r3 = false;
	let mut has_gk12r11 = false;
	let mut has_invalid_gk1a = false;

	for (drug_idx, drug) in validation_ctx.drugs.iter().enumerate() {
		let mut chars = list_drug_characteristics(ctx, mm, drug.id).await?;
		chars.extend(derive_fda_device_characteristics(drug));
		let malfunction_this_drug = chars.iter().any(|ch| {
			characteristic_code_matches(ch.code.as_deref(), "FDA.G.k.12.r.1")
				&& is_truthy_characteristic(ch)
		});
		if combination_true
			&& malfunction_this_drug
			&& drug.drug_characterization == "4"
			&& !has_text(drug.fda_other_characterization.as_deref())
		{
			push_issue_by_code(
				issues,
				"FDA.G.k.1.a.REQUIRED",
				format!("drugs.{drug_idx}.fdaOtherCharacterization"),
			);
		}
		if local_criteria == Some("5") && !malfunction_this_drug {
			push_issue_by_code(
				issues,
				"FDA.G.k.12.r.1.REQUIRED",
				format!("drugs.{drug_idx}.fdaDeviceInfo.malfunction"),
			);
		}
		if malfunction_this_drug {
			if !has_characteristic_value(&chars, "FDA.G.k.12.r.4") {
				push_issue_by_code(
					issues,
					"FDA.G.k.12.r.4.REQUIRED",
					format!("drugs.{drug_idx}.fdaDeviceInfo.deviceBrandName"),
				);
			}
			if !has_characteristic_value(&chars, "FDA.G.k.12.r.5") {
				push_issue_by_code(
					issues,
					"FDA.G.k.12.r.5.REQUIRED",
					format!("drugs.{drug_idx}.fdaDeviceInfo.commonDeviceName"),
				);
			}
			if !has_characteristic_value(&chars, "FDA.G.k.12.r.6") {
				push_issue_by_code(
					issues,
					"FDA.G.k.12.r.6.REQUIRED",
					format!("drugs.{drug_idx}.fdaDeviceInfo.deviceProductCode"),
				);
			}
		}
		if malfunction_this_drug {
			has_malfunction_any = true;
			if drug.drug_characterization == "1" {
				has_malfunction_suspect = true;
			}
		}
		if chars.iter().any(|ch| {
			characteristic_code_matches(ch.code.as_deref(), "FDA.G.k.12.r.3")
		}) {
			has_gk12r3 = true;
		}
		if chars.iter().any(|ch| {
			characteristic_code_matches(ch.code.as_deref(), "FDA.G.k.12.r.11")
		}) {
			has_gk12r11 = true;
		}
		let has_gk1a_one = chars.iter().any(|ch| {
			characteristic_code_matches(ch.code.as_deref(), "FDA.G.k.1.a")
				&& is_code_one_characteristic(ch)
		});
		if has_gk1a_one
			&& !(combination_true
				&& malfunction_this_drug
				&& drug.drug_characterization == "4")
		{
			has_invalid_gk1a = true;
		}
	}

	if local_criteria == Some("5") && !has_malfunction_suspect {
		push_issue_by_code(
			issues,
			"FDA.G.K.12.REQUIRED",
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if has_malfunction_any && !has_gk12r3 {
		push_issue_by_code(
			issues,
			"FDA.G.k.12.r.3.r.REQUIRED",
			"drugs.0.fdaDeviceInfo.deviceProblemCodes",
		);
		push_issue_by_code(
			issues,
			"FDA.G.K.12.R.3.REQUIRED",
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if local_criteria == Some("4") && has_malfunction_any && !has_gk12r11 {
		push_issue_by_code(
			issues,
			"FDA.G.k.12.r.11.r.REQUIRED",
			"drugs.0.fdaDeviceInfo.remedialActions",
		);
		push_issue_by_code(
			issues,
			"FDA.G.K.12.R.11.REQUIRED",
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if has_invalid_gk1a {
		push_issue_by_code(
			issues,
			"FDA.G.K.1.A.CONDITIONAL",
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	Ok(())
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
	eval_indexed_vocabulary_variants(
		issues,
		&validation_ctx.drugs,
		G_MFDS_PRODUCT_VOCABULARY_RULES,
		vocabulary_receiver,
		&validation_ctx.vocabulary,
	);
	let receiver_is_ct_or_cu = is_mfds_clinical_trial_receiver(msg_receiver)
		|| is_mfds_compassionate_use_receiver(msg_receiver);

	let mut domestic_drug_ids = HashSet::new();
	let mut drug_index_by_id = HashMap::new();
	let mut drug_has_mfds_mpid_by_id = HashMap::new();

	let drug_views = validation_ctx
		.drugs
		.iter()
		.enumerate()
		.map(|(idx, drug)| {
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
			MfdsDrugRuleView {
				index: idx,
				mpid: drug.mfds_mpid.clone(),
				mpid_version: drug.mfds_mpid_version.clone(),
				facts: RuleFacts {
					mfds_product_code_required_context: Some(
						receiver_is_kr || receiver_is_fr,
					),
					mfds_product_version_required_context: Some(
						receiver_is_fr && has_mfds_mpid,
					),
					mfds_drug_domestic_kr: Some(is_domestic_kr),
					mfds_drug_foreign_non_kr: Some(is_foreign_non_kr),
					..RuleFacts::default()
				},
			}
		})
		.collect::<Vec<_>>();
	eval_catalog_values(issues, &drug_views, G_MFDS_DRUG_CATALOG_VALUE_RULES);

	let substance_views = mfds_ctx
		.active_substances
		.iter()
		.filter_map(|substance| {
			let (drug_index, substance_index) = resolve_drug_child_indices(
				&drug_index_by_id,
				substance.drug_id,
				substance.sequence_number,
			)?;
			let drug_has_mfds_mpid = drug_has_mfds_mpid_by_id
				.get(&substance.drug_id)
				.copied()
				.unwrap_or(false);
			Some(MfdsSubstanceRuleView {
				drug_index,
				substance_index,
				id: substance.mfds_id.clone(),
				version: substance.mfds_version.clone(),
				facts: RuleFacts {
					mfds_drug_domestic_kr: Some(
						domestic_drug_ids.contains(&substance.drug_id),
					),
					mfds_substance_code_required_context: Some(
						(receiver_is_kr || receiver_is_fr) && !drug_has_mfds_mpid,
					),
					mfds_substance_version_required_context: Some(
						receiver_is_fr && has_text(substance.mfds_id.as_deref()),
					),
					..RuleFacts::default()
				},
			})
		})
		.collect::<Vec<_>>();
	eval_catalog_values(
		issues,
		&substance_views,
		G_MFDS_SUBSTANCE_CATALOG_VALUE_RULES,
	);

	let relatedness_views = mfds_ctx
		.relatedness
		.iter()
		.filter_map(|r| {
			let (drug_index, assessment_index) = resolve_drug_child_indices(
				&drug_index_by_id,
				r.drug_id,
				r.relatedness_sequence_number,
			)?;
			let has_source = has_text(r.source_of_assessment.as_deref());
			let has_method = has_text(r.method_of_assessment.as_deref());
			let has_result_kr1 = has_text(r.result_of_assessment.as_deref());
			let has_result_kr2 = has_text(r.result_of_assessment_kr2.as_deref());
			let has_any_result = has_result_kr1 || has_result_kr2;
			let method_code = r.method_of_assessment.as_deref().map(str::trim);
			let method_is_who_umc = method_code == Some("1");
			let method_is_krct = method_code == Some("2");
			let method_required_context = has_source || receiver_is_ct_or_cu;
			let kr2_required_context = has_source
				&& method_is_krct
				&& (report_type_is_study || receiver_is_ct_or_cu);
			Some(MfdsRelatednessRuleView {
				drug_index,
				assessment_index,
				source: r.source_of_assessment.clone(),
				method: r.method_of_assessment.clone(),
				result_kr1: r.result_of_assessment.clone(),
				result_kr2: r.result_of_assessment_kr2.clone(),
				facts: RuleFacts {
					mfds_relatedness_method_required_context: Some(
						method_required_context,
					),
					mfds_relatedness_kr1_required_context: Some(
						has_source && method_is_who_umc,
					),
					mfds_relatedness_kr2_required_context: Some(
						kr2_required_context,
					),
					mfds_relatedness_method_present: Some(has_method),
					mfds_relatedness_result_present: Some(has_any_result),
					..RuleFacts::default()
				},
			})
		})
		.collect::<Vec<_>>();
	eval_catalog_values(
		issues,
		&relatedness_views,
		G_MFDS_RELATEDNESS_CATALOG_VALUE_RULES,
	);

	for relatedness in &relatedness_views {
		let method_code = relatedness.method.as_deref().map(str::trim);
		if let Some(code) = method_code {
			let valid_code = code == "1" || code == "2";
			let profile_valid = if receiver_is_ct_or_cu {
				code == "2"
			} else if receiver_is_kr {
				code == "1"
			} else if receiver_is_fr {
				false
			} else {
				true
			};
			if !valid_code || !profile_valid {
				push_issue_by_code(
					issues,
					"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
					relatedness.path("methodOfAssessment"),
				);
			}
		}
		// G.k.9.i.2.r.3.KR.1 allowed values: WHO-UMC result must be 1..6 or the
		// NA nullFlavor token. Only enforced when the method is WHO-UMC (1).
		if method_code == Some("1") {
			if let Some(result_code) =
				relatedness.result_kr1.as_deref().map(str::trim)
			{
				if !result_code.is_empty()
					&& !matches!(
						result_code,
						"1" | "2" | "3" | "4" | "5" | "6" | "NA"
					) {
					push_issue_by_code(
						issues,
						"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
						relatedness.path("resultOfAssessment"),
					);
				}
			}
		}
	}
}

#[cfg(test)]
pub(super) fn constraint_rule_codes() -> Vec<&'static str> {
	G_DRUG_CONSTRAINT_RULES
		.iter()
		.map(|rule| rule.code)
		.chain(
			G_REACTION_ASSESSMENT_CONSTRAINT_RULES
				.iter()
				.map(|rule| rule.code),
		)
		.chain(
			G_ACTIVE_SUBSTANCE_CONSTRAINT_RULES
				.iter()
				.map(|rule| rule.code),
		)
		.chain(super::rule_table::nested_meddra_constraint_codes(
			G_INDICATION_MEDDRA_RULES,
		))
		.collect()
}

#[cfg(test)]
pub(super) fn table_rule_codes() -> Vec<&'static str> {
	let mut codes = Vec::new();
	macro_rules! add {
		($rules:expr) => {
			codes.extend(super::rule_table::table_rule_codes($rules));
		};
	}
	add!(G_MFDS_PRODUCT_VOCABULARY_RULES);
	add!(G_DRUG_VALUE_RULES);
	add!(G_DRUG_LENGTH_RULES);
	add!(G_DRUG_DERIVED_LENGTH_RULES);
	add!(G_DRUG_CONSTRAINT_RULES);
	add!(G_DRUG_COMPANION_RULES);
	add!(G_ACTIVE_SUBSTANCE_LENGTH_RULES);
	add!(G_ACTIVE_SUBSTANCE_CONSTRAINT_RULES);
	add!(G_ACTIVE_SUBSTANCE_DERIVED_LENGTH_RULES);
	add!(G_ACTIVE_SUBSTANCE_COMPANION_RULES);
	add!(G_DOSAGE_LENGTH_RULES);
	add!(G_DOSAGE_DERIVED_LENGTH_RULES);
	add!(G_DOSAGE_COMPANION_RULES);
	add!(G_DOSAGE_FUTURE_DATE_RULES);
	add!(G_INDICATION_LENGTH_RULES);
	add!(G_INDICATION_COMPANION_RULES);
	add!(G_REACTION_ASSESSMENT_LENGTH_RULES);
	add!(G_REACTION_ASSESSMENT_DERIVED_LENGTH_RULES);
	add!(G_RELATEDNESS_ASSESSMENT_LENGTH_RULES);
	add!(G_REACTION_ASSESSMENT_CONSTRAINT_RULES);
	add!(G_REACTION_ASSESSMENT_COMPANION_RULES);
	add!(G_MFDS_DRUG_CATALOG_VALUE_RULES);
	add!(G_MFDS_SUBSTANCE_CATALOG_VALUE_RULES);
	add!(G_MFDS_RELATEDNESS_CATALOG_VALUE_RULES);
	codes.extend(super::rule_table::nested_meddra_rule_codes(
		G_INDICATION_MEDDRA_RULES,
	));
	codes
}

#[cfg(test)]
pub(super) fn direct_rule_codes() -> &'static [&'static str] {
	&[
		"FDA.G.K.1.A.CONDITIONAL",
		"FDA.G.K.12.R.11.REQUIRED",
		"FDA.G.K.12.R.3.REQUIRED",
		"FDA.G.K.12.REQUIRED",
		"FDA.G.k.1.a.REQUIRED",
		"FDA.G.k.12.r.1.REQUIRED",
		"FDA.G.k.12.r.11.r.REQUIRED",
		"FDA.G.k.12.r.3.r.REQUIRED",
		"FDA.G.k.12.r.4.REQUIRED",
		"FDA.G.k.12.r.5.REQUIRED",
		"FDA.G.k.12.r.6.REQUIRED",
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
	]
}

#[cfg(test)]
mod conditioned_catalog_rule_tests {
	use super::*;
	use sqlx::types::Uuid;

	#[test]
	fn drug_rules_cover_domestic_foreign_and_unrelated_contexts() {
		let rows = [
			MfdsDrugRuleView {
				index: 1,
				mpid: None,
				mpid_version: None,
				facts: RuleFacts {
					mfds_product_code_required_context: Some(true),
					mfds_product_version_required_context: Some(false),
					mfds_drug_domestic_kr: Some(true),
					mfds_drug_foreign_non_kr: Some(false),
					..RuleFacts::default()
				},
			},
			MfdsDrugRuleView {
				index: 2,
				mpid: Some("product".to_string()),
				mpid_version: None,
				facts: RuleFacts {
					mfds_product_code_required_context: Some(true),
					mfds_product_version_required_context: Some(true),
					mfds_drug_domestic_kr: Some(false),
					mfds_drug_foreign_non_kr: Some(true),
					..RuleFacts::default()
				},
			},
			MfdsDrugRuleView {
				index: 3,
				mpid: None,
				mpid_version: None,
				facts: RuleFacts::default(),
			},
		];
		let mut issues = Vec::new();
		eval_catalog_values(&mut issues, &rows, G_MFDS_DRUG_CATALOG_VALUE_RULES);

		assert_eq!(
			issues
				.iter()
				.map(|issue| issue.code.as_str())
				.collect::<Vec<_>>(),
			[
				"MFDS.G.k.2.1.KR.1b.REQUIRED",
				"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
				"MFDS.G.k.2.1.KR.1a.REQUIRED",
			]
		);
		assert!(issues
			.iter()
			.all(|issue| issue.field_path.as_deref() != Some("drugs.3.mfdsMpid")));
	}

	#[test]
	fn substance_rules_preserve_resolved_drug_and_substance_indices() {
		let rows = [
			MfdsSubstanceRuleView {
				drug_index: 1,
				substance_index: 2,
				id: None,
				version: None,
				facts: RuleFacts {
					mfds_drug_domestic_kr: Some(true),
					mfds_substance_code_required_context: Some(true),
					mfds_substance_version_required_context: Some(false),
					..RuleFacts::default()
				},
			},
			MfdsSubstanceRuleView {
				drug_index: 3,
				substance_index: 4,
				id: Some("ingredient".to_string()),
				version: None,
				facts: RuleFacts {
					mfds_drug_domestic_kr: Some(false),
					mfds_substance_code_required_context: Some(true),
					mfds_substance_version_required_context: Some(true),
					..RuleFacts::default()
				},
			},
		];
		let mut issues = Vec::new();
		eval_catalog_values(
			&mut issues,
			&rows,
			G_MFDS_SUBSTANCE_CATALOG_VALUE_RULES,
		);

		assert_eq!(issues.len(), 3);
		assert_eq!(
			issues
				.iter()
				.map(|issue| issue.field_path.as_deref().unwrap())
				.collect::<Vec<_>>(),
			[
				"drugs.1.activeSubstances.2.mfdsId",
				"drugs.1.activeSubstances.2.mfdsId",
				"drugs.3.activeSubstances.4.mfdsVersion",
			]
		);
	}

	#[test]
	fn relatedness_rules_cover_method_results_and_source_companion() {
		let rows = [
			MfdsRelatednessRuleView {
				drug_index: 1,
				assessment_index: 2,
				source: Some("source".to_string()),
				method: None,
				result_kr1: None,
				result_kr2: None,
				facts: RuleFacts {
					mfds_relatedness_method_required_context: Some(true),
					..RuleFacts::default()
				},
			},
			MfdsRelatednessRuleView {
				drug_index: 1,
				assessment_index: 3,
				source: Some("source".to_string()),
				method: Some("1".to_string()),
				result_kr1: None,
				result_kr2: None,
				facts: RuleFacts {
					mfds_relatedness_method_required_context: Some(true),
					mfds_relatedness_kr1_required_context: Some(true),
					..RuleFacts::default()
				},
			},
			MfdsRelatednessRuleView {
				drug_index: 1,
				assessment_index: 4,
				source: Some("source".to_string()),
				method: Some("2".to_string()),
				result_kr1: None,
				result_kr2: None,
				facts: RuleFacts {
					mfds_relatedness_method_required_context: Some(true),
					mfds_relatedness_kr2_required_context: Some(true),
					..RuleFacts::default()
				},
			},
			MfdsRelatednessRuleView {
				drug_index: 1,
				assessment_index: 5,
				source: None,
				method: Some("1".to_string()),
				result_kr1: None,
				result_kr2: None,
				facts: RuleFacts {
					mfds_relatedness_method_present: Some(true),
					mfds_relatedness_result_present: Some(false),
					..RuleFacts::default()
				},
			},
		];
		let mut issues = Vec::new();
		eval_catalog_values(
			&mut issues,
			&rows,
			G_MFDS_RELATEDNESS_CATALOG_VALUE_RULES,
		);

		assert_eq!(
			issues
				.iter()
				.map(|issue| issue.code.as_str())
				.collect::<Vec<_>>(),
			[
				"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
				"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
				"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
				"MFDS.G.k.9.i.2.r.1.REQUIRED",
			]
		);
	}

	#[test]
	fn child_indices_have_no_owner_or_sequence_fallback() {
		let known_drug = Uuid::new_v4();
		let unknown_drug = Uuid::new_v4();
		let indices = HashMap::from([(known_drug, 2)]);

		assert_eq!(
			resolve_drug_child_indices(&indices, known_drug, 4),
			Some((2, 3))
		);
		assert_eq!(resolve_drug_child_indices(&indices, unknown_drug, 4), None);
		assert_eq!(resolve_drug_child_indices(&indices, known_drug, 0), None);
	}
}

#[cfg(test)]
mod golden_g_required_tests {
	use super::*;
	use lib_core::model::case::Case;
	use serde_json::json;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Decimal;
	use sqlx::types::Uuid;

	fn dummy_case() -> Case {
		Case {
			id: Uuid::nil(),
			organization_id: Uuid::nil(),
			dg_prd_key: None,
			status: String::new(),
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

	fn empty_ctx() -> ValidationContext {
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

	fn drug() -> DrugInformation {
		DrugInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			source_product_presave_id: None,
			sequence_number: 1,
			drug_characterization: String::new(),
			medicinal_product: String::new(),
			mpid: None,
			mpid_version: None,
			mfds_mpid_version: None,
			mfds_mpid: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			brand_name: None,
			drug_generic_name: None,
			drug_authorization_number: None,
			manufacturer_name: None,
			manufacturer_country: None,
			batch_lot_number: None,
			cumulative_dose_first_reaction_value: None,
			cumulative_dose_first_reaction_unit: None,
			gestation_period_exposure_value: None,
			gestation_period_exposure_unit: None,
			dosage_text: None,
			action_taken: None,
			rechallenge: None,
			parent_dosage_text: None,
			fda_additional_info_coded: None,
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
			fda_other_characterization: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn substance() -> DrugActiveSubstance {
		DrugActiveSubstance {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			sequence_number: 1,
			substance_name: None,
			substance_termid: None,
			substance_termid_version: None,
			mfds_version: None,
			mfds_id: None,
			strength_value: None,
			strength_unit: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn dosage() -> DosageInformation {
		DosageInformation {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_value: None,
			frequency_unit: None,
			first_administration_date: None,
			first_administration_time: None,
			last_administration_date: None,
			last_administration_time: None,
			duration_value: None,
			duration_unit: None,
			continuing: None,
			batch_lot_number: None,
			batch_lot_number_null_flavor: None,
			dosage_text: None,
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: None,
			route_termid: None,
			route_termid_version: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn indication() -> DrugIndication {
		DrugIndication {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			sequence_number: 1,
			indication_text: None,
			indication_text_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn assessment() -> DrugReactionAssessment {
		DrugReactionAssessment {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			reaction_id: Uuid::nil(),
			administration_start_interval_value: None,
			administration_start_interval_unit: None,
			last_dose_interval_value: None,
			last_dose_interval_unit: None,
			recurrence_action: None,
			recurrence_meddra_version: None,
			recurrence_meddra_code: None,
			reaction_recurred: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn relatedness() -> RelatednessAssessment {
		RelatednessAssessment {
			id: Uuid::nil(),
			drug_reaction_assessment_id: Uuid::nil(),
			sequence_number: 1,
			source_of_assessment: None,
			method_of_assessment: None,
			result_of_assessment: None,
			result_of_assessment_kr2: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn codes_for(ctx: &ValidationContext) -> Vec<String> {
		let mut issues = Vec::new();
		collect_ich_issues(ctx, &mut issues);
		issues.into_iter().map(|issue| issue.code).collect()
	}

	fn length_issues(ctx: &ValidationContext) -> Vec<(String, String)> {
		let mut issues = Vec::new();
		collect_ich_issues(ctx, &mut issues);
		issues
			.into_iter()
			.filter(|issue| issue.code.ends_with(".LENGTH.MAX"))
			.map(|issue| (issue.code, issue.field_path.unwrap_or_default()))
			.collect()
	}

	#[test]
	fn allowed_value_rules_cover_g_drug_and_reaction_codes() {
		let mut ctx = empty_ctx();
		let mut drug = drug();
		drug.id = Uuid::from_u128(1);
		drug.drug_characterization = "9".to_string();
		drug.investigational_product_blinded = Some(false);
		drug.action_taken = Some("8".to_string());
		drug.drug_additional_info_codes_json = Some(json!([
			{ "value_code": "12" }
		]));
		ctx.drugs.push(drug);

		let mut assessment = assessment();
		assessment.drug_id = Uuid::from_u128(1);
		assessment.reaction_recurred = Some("9".to_string());
		ctx.drug_reaction_assessments.push(assessment);

		let codes = codes_for(&ctx);
		assert!(codes.contains(&"ICH.G.k.1.ALLOWED.VALUE".to_string()));
		assert!(codes.contains(&"ICH.G.k.2.5.ALLOWED.VALUE".to_string()));
		assert!(codes.contains(&"ICH.G.k.8.ALLOWED.VALUE".to_string()));
		assert!(codes.contains(&"ICH.G.k.9.i.4.ALLOWED.VALUE".to_string()));
		assert!(codes.contains(&"ICH.G.k.10.r.ALLOWED.VALUE".to_string()));
	}

	#[test]
	fn meddra_vocabulary_rules_cover_g_indication_codes() {
		let mut ctx = empty_ctx();
		ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);

		let mut drug = drug();
		drug.id = Uuid::from_u128(1);
		ctx.drugs.push(drug);

		let mut indication = indication();
		indication.drug_id = Uuid::from_u128(1);
		indication.indication_meddra_version = Some("99.9".to_string());
		indication.indication_meddra_code = Some("99999999".to_string());
		ctx.indications.push(indication);

		let codes = codes_for(&ctx);
		assert!(codes.contains(&"ICH.G.k.7.r.2a.VOCABULARY".to_string()));
		assert!(codes.contains(&"ICH.G.k.7.r.2b.VOCABULARY".to_string()));
	}

	#[test]
	fn empty_drug_collection_flags_placeholder_drug_rules() {
		assert_eq!(
			codes_for(&empty_ctx()),
			vec![
				"ICH.G.k.1.REQUIRED".to_string(),
				"ICH.G.k.2.2.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn drug_required_and_pair_rules_are_preserved() {
		let mut ctx = empty_ctx();
		let mut drug = drug();
		drug.cumulative_dose_first_reaction_unit = Some("mg".to_string());
		drug.gestation_period_exposure_value = Some("1".parse().unwrap());
		ctx.drugs.push(drug);

		assert_eq!(
			codes_for(&ctx),
			vec![
				"ICH.G.k.1.REQUIRED".to_string(),
				"ICH.G.k.2.2.REQUIRED".to_string(),
				"ICH.G.k.5a.REQUIRED".to_string(),
				"ICH.G.k.6b.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn nested_collection_companion_rules_are_preserved() {
		let mut ctx = empty_ctx();
		let mut substance = substance();
		substance.substance_termid = Some("SUB123".to_string());
		substance.strength_value = Some("1".parse().unwrap());
		ctx.active_substances.push(substance);

		let mut dosage = dosage();
		dosage.dose_value = Some("1".parse().unwrap());
		dosage.duration_unit = Some("d".to_string());
		dosage.route_of_administration = Some("030".to_string());
		ctx.dosages.push(dosage);

		assert_eq!(
			codes_for(&ctx),
			vec![
				"ICH.G.k.1.REQUIRED".to_string(),
				"ICH.G.k.2.2.REQUIRED".to_string(),
				"ICH.G.k.2.3.r.2a.REQUIRED".to_string(),
				"ICH.G.k.2.3.r.3b.REQUIRED".to_string(),
				"ICH.G.k.4.r.1b.REQUIRED".to_string(),
				"ICH.G.k.4.r.6a.REQUIRED".to_string(),
				"ICH.G.k.4.r.10.2a.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn indication_and_reaction_assessment_pair_rules_are_preserved() {
		let mut ctx = empty_ctx();
		let mut indication = indication();
		indication.indication_meddra_version = Some("26.1".to_string());
		ctx.indications.push(indication);

		let mut assessment = assessment();
		assessment.administration_start_interval_value = Some("1".parse().unwrap());
		assessment.last_dose_interval_unit = Some("d".to_string());
		ctx.drug_reaction_assessments.push(assessment);

		assert_eq!(
			codes_for(&ctx),
			vec![
				"ICH.G.k.1.REQUIRED".to_string(),
				"ICH.G.k.2.2.REQUIRED".to_string(),
				"ICH.G.k.7.r.2b.REQUIRED".to_string(),
				"ICH.G.k.9.i.3.1b.REQUIRED".to_string(),
				"ICH.G.k.9.i.3.2a.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn max_length_rules_cover_g_drug_fields() {
		let mut ctx = empty_ctx();
		let mut drug = drug();
		drug.drug_characterization = "12".to_string();
		drug.mpid_version = Some("12345678901".to_string());
		drug.mpid = Some("x".repeat(1001));
		drug.phpid_version = Some("12345678901".to_string());
		drug.phpid = Some("x".repeat(251));
		drug.medicinal_product = "x".repeat(251);
		drug.obtain_drug_country = Some("USA".to_string());
		drug.drug_authorization_number = Some("x".repeat(36));
		drug.manufacturer_country = Some("USA".to_string());
		drug.manufacturer_name = Some("x".repeat(61));
		drug.cumulative_dose_first_reaction_value =
			Some(Decimal::new(12_345_678_901, 0));
		drug.cumulative_dose_first_reaction_unit = Some("x".repeat(51));
		drug.gestation_period_exposure_value = Some(Decimal::new(1234, 0));
		drug.gestation_period_exposure_unit = Some("x".repeat(51));
		drug.action_taken = Some("12".to_string());
		drug.drug_additional_info_codes_json = Some(json!([
			{ "value_code": "123" }
		]));
		drug.drug_additional_information = Some("x".repeat(2001));
		ctx.drugs.push(drug);

		assert_eq!(
			length_issues(&ctx),
			vec![
				(
					"ICH.G.k.1.LENGTH.MAX".to_string(),
					"drugs.0.drugCharacterization".to_string()
				),
				(
					"ICH.G.k.2.1.1a.LENGTH.MAX".to_string(),
					"drugs.0.mpidVersion".to_string()
				),
				(
					"ICH.G.k.2.1.1b.LENGTH.MAX".to_string(),
					"drugs.0.mpid".to_string()
				),
				(
					"ICH.G.k.2.1.2a.LENGTH.MAX".to_string(),
					"drugs.0.phpidVersion".to_string()
				),
				(
					"ICH.G.k.2.1.2b.LENGTH.MAX".to_string(),
					"drugs.0.phpid".to_string()
				),
				(
					"ICH.G.k.2.2.LENGTH.MAX".to_string(),
					"drugs.0.medicinalProduct".to_string()
				),
				(
					"ICH.G.k.2.4.LENGTH.MAX".to_string(),
					"drugs.0.obtainDrugCountry".to_string()
				),
				(
					"ICH.G.k.3.1.LENGTH.MAX".to_string(),
					"drugs.0.drugAuthorizationNumber".to_string()
				),
				(
					"ICH.G.k.3.2.LENGTH.MAX".to_string(),
					"drugs.0.drugAuthorizationCountry".to_string()
				),
				(
					"ICH.G.k.3.3.LENGTH.MAX".to_string(),
					"drugs.0.manufacturerName".to_string()
				),
				(
					"ICH.G.k.5b.LENGTH.MAX".to_string(),
					"drugs.0.cumulativeDoseFirstReactionUnit".to_string()
				),
				(
					"ICH.G.k.6b.LENGTH.MAX".to_string(),
					"drugs.0.gestationPeriodExposureUnit".to_string()
				),
				(
					"ICH.G.k.8.LENGTH.MAX".to_string(),
					"drugs.0.actionTaken".to_string()
				),
				(
					"ICH.G.k.11.LENGTH.MAX".to_string(),
					"drugs.0.drugAdditionalInformation".to_string()
				),
				(
					"ICH.G.k.5a.LENGTH.MAX".to_string(),
					"drugs.0.cumulativeDoseFirstReactionValue".to_string()
				),
				(
					"ICH.G.k.6a.LENGTH.MAX".to_string(),
					"drugs.0.gestationPeriodExposureValue".to_string()
				),
				(
					"ICH.G.k.10.r.LENGTH.MAX".to_string(),
					"drugs.0.drugAdditionalInformationCodes".to_string()
				),
			]
		);
	}

	#[test]
	fn max_length_rules_cover_g_nested_drug_collections() {
		let mut ctx = empty_ctx();
		let mut drug = drug();
		drug.id = Uuid::from_u128(1);
		ctx.drugs.push(drug);

		let mut substance = substance();
		substance.drug_id = Uuid::from_u128(1);
		substance.substance_name = Some("x".repeat(251));
		substance.substance_termid_version = Some("x".repeat(11));
		substance.substance_termid = Some("x".repeat(101));
		substance.strength_value = Some(Decimal::new(12_345_678_901, 0));
		substance.strength_unit = Some("x".repeat(51));
		ctx.active_substances.push(substance);

		let mut dosage = dosage();
		dosage.drug_id = Uuid::from_u128(1);
		dosage.dose_value = Some(Decimal::new(123_456_789, 0));
		dosage.dose_unit = Some("x".repeat(51));
		dosage.number_of_units = Some(12_345);
		dosage.frequency_unit = Some("x".repeat(51));
		dosage.duration_value = Some(Decimal::new(123_456, 0));
		dosage.duration_unit = Some("x".repeat(51));
		dosage.batch_lot_number = Some("x".repeat(36));
		dosage.dosage_text = Some("x".repeat(2001));
		dosage.dose_form = Some("x".repeat(61));
		dosage.dose_form_termid_version = Some("x".repeat(11));
		dosage.dose_form_termid = Some("x".repeat(101));
		dosage.route_of_administration = Some("x".repeat(61));
		dosage.route_termid_version = Some("x".repeat(11));
		dosage.route_termid = Some("x".repeat(101));
		dosage.parent_route = Some("x".repeat(61));
		dosage.parent_route_termid_version = Some("x".repeat(11));
		dosage.parent_route_termid = Some("x".repeat(101));
		ctx.dosages.push(dosage);

		let mut indication = indication();
		indication.drug_id = Uuid::from_u128(1);
		indication.indication_text = Some("x".repeat(251));
		indication.indication_meddra_version = Some("x".repeat(5));
		indication.indication_meddra_code = Some("x".repeat(9));
		ctx.indications.push(indication);

		let mut assessment = assessment();
		assessment.id = Uuid::from_u128(2);
		assessment.drug_id = Uuid::from_u128(1);
		assessment.administration_start_interval_value =
			Some(Decimal::new(123_456, 0));
		assessment.administration_start_interval_unit = Some("x".repeat(51));
		assessment.last_dose_interval_value = Some(Decimal::new(123_456, 0));
		assessment.last_dose_interval_unit = Some("x".repeat(51));
		assessment.reaction_recurred = Some("12".to_string());
		ctx.drug_reaction_assessments.push(assessment);

		let mut relatedness = relatedness();
		relatedness.drug_reaction_assessment_id = Uuid::from_u128(2);
		relatedness.source_of_assessment = Some("x".repeat(61));
		relatedness.method_of_assessment = Some("x".repeat(61));
		relatedness.result_of_assessment = Some("x".repeat(61));
		ctx.relatedness_assessments.push(relatedness);

		assert_eq!(length_issues(&ctx).len(), 33);
		assert!(length_issues(&ctx).contains(&(
			"ICH.G.k.2.3.r.1.LENGTH.MAX".to_string(),
			"drugs.0.activeSubstances.0.substanceName".to_string()
		)));
		assert!(length_issues(&ctx).contains(&(
			"ICH.G.k.4.r.1a.LENGTH.MAX".to_string(),
			"drugs.0.dosages.0.doseValue".to_string()
		)));
		assert!(length_issues(&ctx).contains(&(
			"ICH.G.k.7.r.2b.LENGTH.MAX".to_string(),
			"drugs.0.indications.0.indicationMeddraCode".to_string()
		)));
		assert!(length_issues(&ctx).contains(&(
			"ICH.G.k.9.i.4.LENGTH.MAX".to_string(),
			"drugs.0.reactionAssessments.0.reactionRecurred".to_string()
		)));
		assert!(length_issues(&ctx).contains(&(
			"ICH.G.k.9.i.2.r.1.LENGTH.MAX".to_string(),
			"drugs.0.reactionAssessments.0.relatednessAssessments.0.sourceOfAssessment"
				.to_string()
		)));
	}
}
