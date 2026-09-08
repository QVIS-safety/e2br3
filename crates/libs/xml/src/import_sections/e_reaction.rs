// Section E importer (Reaction/Event) - FDA mapping.

use crate::error::Error;
use crate::import_constraint;
use crate::import_sections::shared::{
	parse_bool_literal, parse_xml_id_opt, ImportIdMap,
};
use crate::mapping::fda::e_reaction::EReactionPaths;
use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::model::ModelManager;
use libxml::parser::Parser;
use libxml::tree::Node;
use libxml::xpath::Context;
use sqlx::types::Uuid;

#[derive(Debug)]
pub struct EReactionImport {
	pub xml_id: Option<String>,
	pub primary_source_reaction: Option<String>,
	pub primary_source_reaction_translation: Option<String>,
	pub reaction_language: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub term_highlighted: Option<String>,
	pub serious: Option<bool>,
	pub criteria_death: Option<bool>,
	pub criteria_death_null_flavor: Option<String>,
	pub criteria_life_threatening: Option<bool>,
	pub criteria_life_threatening_null_flavor: Option<String>,
	pub criteria_hospitalization: Option<bool>,
	pub criteria_hospitalization_null_flavor: Option<String>,
	pub criteria_disabling: Option<bool>,
	pub criteria_disabling_null_flavor: Option<String>,
	pub criteria_congenital_anomaly: Option<bool>,
	pub criteria_congenital_anomaly_null_flavor: Option<String>,
	pub criteria_other_medically_important: Option<bool>,
	pub criteria_other_medically_important_null_flavor: Option<String>,
	pub required_intervention: Option<bool>,
	pub required_intervention_null_flavor: Option<String>,
	pub expectedness: Option<String>,
	pub severity: Option<String>,
	pub mfds_device_ae_classification: Option<String>,
	pub mfds_device_ae_outcome: Option<String>,
	pub mfds_device_cause_medical_device: Option<bool>,
	pub mfds_device_cause_procedure_issue: Option<bool>,
	pub mfds_device_cause_patient_condition: Option<bool>,
	pub mfds_device_cause_unable_to_assess: Option<bool>,
	pub mfds_device_cause_other: Option<String>,
	pub mfds_device_action_reason: Option<String>,
	pub mfds_device_action_recall: Option<bool>,
	pub mfds_device_action_repair: Option<bool>,
	pub mfds_device_action_inspection: Option<bool>,
	pub mfds_device_action_replacement: Option<bool>,
	pub mfds_device_action_improvement: Option<bool>,
	pub mfds_device_action_monitoring: Option<bool>,
	pub mfds_device_action_notification: Option<bool>,
	pub mfds_device_action_label_change: Option<bool>,
	pub mfds_device_action_other: Option<String>,
	pub start_date: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub end_date: Option<String>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<String>,
	pub duration_unit: Option<String>,
	pub outcome: Option<String>,
	pub medical_confirmation: Option<bool>,
	pub country_code: Option<String>,
}

pub fn parse_e_reactions(xml: &[u8]) -> Result<Vec<EReactionImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let _ =
		xpath.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");

	let nodes = xpath
		.findnodes(EReactionPaths::REACTION_NODE, None)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query reactions".to_string(),
			line: None,
			column: None,
		})?;

	let mut imports: Vec<EReactionImport> = Vec::new();
	for node in nodes {
		let xml_id = parse_xml_id_opt(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::XML_ID_ROOT,
		));
		let translation_text = read_e_i_1_1b(&mut xpath, &node)?;
		let primary = read_e_i_1_1a(&mut xpath, &node)?;
		let reaction_meddra_version = read_e_i_2_1a(&mut xpath, &node)?;
		let reaction_meddra_code = read_e_i_2_1b(&mut xpath, &node)?;
		let reaction_language = read_e_i_1_2(&mut xpath, &node)?;
		let term_highlighted = read_e_i_3_1(&mut xpath, &node)?;
		let (criteria_death, criteria_death_null_flavor) =
			read_e_i_3_2a(&mut xpath, &node)?;
		let (criteria_life_threatening, criteria_life_threatening_null_flavor) =
			read_e_i_3_2b(&mut xpath, &node)?;
		let (criteria_hospitalization, criteria_hospitalization_null_flavor) =
			read_e_i_3_2c(&mut xpath, &node)?;
		let (criteria_disabling, criteria_disabling_null_flavor) =
			read_e_i_3_2d(&mut xpath, &node)?;
		let (criteria_congenital_anomaly, criteria_congenital_anomaly_null_flavor) =
			read_e_i_3_2e(&mut xpath, &node)?;
		let (
			criteria_other_medically_important,
			criteria_other_medically_important_null_flavor,
		) = read_e_i_3_2f(&mut xpath, &node)?;
		let serious = read_e_i_serious([
			criteria_death,
			criteria_life_threatening,
			criteria_hospitalization,
			criteria_disabling,
			criteria_congenital_anomaly,
			criteria_other_medically_important,
		]);

		let (required_intervention, required_intervention_null_flavor) =
			read_fda_e_i_3_2h(&mut xpath, &node)?;
		let expectedness = read_e_local_expectedness(&mut xpath, &node);
		let severity = read_e_local_severity(&mut xpath, &node);
		let mfds_device_ae_classification =
			read_e_i_kr_device_ae_classification(&mut xpath, &node);
		let mfds_device_ae_outcome =
			read_e_i_kr_device_ae_outcome(&mut xpath, &node);
		let mfds_device_cause_medical_device =
			read_e_i_kr_device_cause_medical_device(&mut xpath, &node);
		let mfds_device_cause_procedure_issue =
			read_e_i_kr_device_cause_procedure_issue(&mut xpath, &node);
		let mfds_device_cause_patient_condition =
			read_e_i_kr_device_cause_patient_condition(&mut xpath, &node);
		let mfds_device_cause_unable_to_assess =
			read_e_i_kr_device_cause_unable_to_assess(&mut xpath, &node);
		let mfds_device_cause_other =
			read_e_i_kr_device_cause_other(&mut xpath, &node);
		let mfds_device_action_reason =
			read_e_i_kr_device_action_reason(&mut xpath, &node);
		let mfds_device_action_recall =
			read_e_i_kr_device_action_recall(&mut xpath, &node);
		let mfds_device_action_repair =
			read_e_i_kr_device_action_repair(&mut xpath, &node);
		let mfds_device_action_inspection =
			read_e_i_kr_device_action_inspection(&mut xpath, &node);
		let mfds_device_action_replacement =
			read_e_i_kr_device_action_replacement(&mut xpath, &node);
		let mfds_device_action_improvement =
			read_e_i_kr_device_action_improvement(&mut xpath, &node);
		let mfds_device_action_monitoring =
			read_e_i_kr_device_action_monitoring(&mut xpath, &node);
		let mfds_device_action_notification =
			read_e_i_kr_device_action_notification(&mut xpath, &node);
		let mfds_device_action_label_change =
			read_e_i_kr_device_action_label_change(&mut xpath, &node);
		let mfds_device_action_other =
			read_e_i_kr_device_action_other(&mut xpath, &node);
		let (start_date, start_date_null_flavor) = read_e_i_4(&mut xpath, &node)?;
		let (end_date, end_date_null_flavor) = read_e_i_5(&mut xpath, &node)?;
		let duration_value = read_e_i_6a(&mut xpath, &node)?;
		let duration_unit = read_e_i_6b(&mut xpath, &node)?;
		let outcome = read_e_i_7(&mut xpath, &node)?;
		let medical_confirmation = read_e_i_8(&mut xpath, &node)?;
		let country_code = read_e_i_9(&mut xpath, &node)?;

		imports.push(EReactionImport {
			xml_id,
			primary_source_reaction: primary,
			primary_source_reaction_translation: translation_text,
			reaction_language,
			reaction_meddra_version,
			reaction_meddra_code,
			term_highlighted,
			serious,
			criteria_death,
			criteria_death_null_flavor,
			criteria_life_threatening,
			criteria_life_threatening_null_flavor,
			criteria_hospitalization,
			criteria_hospitalization_null_flavor,
			criteria_disabling,
			criteria_disabling_null_flavor,
			criteria_congenital_anomaly,
			criteria_congenital_anomaly_null_flavor,
			criteria_other_medically_important,
			criteria_other_medically_important_null_flavor,
			required_intervention,
			required_intervention_null_flavor,
			expectedness,
			severity,
			mfds_device_ae_classification,
			mfds_device_ae_outcome,
			mfds_device_cause_medical_device,
			mfds_device_cause_procedure_issue,
			mfds_device_cause_patient_condition,
			mfds_device_cause_unable_to_assess,
			mfds_device_cause_other,
			mfds_device_action_reason,
			mfds_device_action_recall,
			mfds_device_action_repair,
			mfds_device_action_inspection,
			mfds_device_action_replacement,
			mfds_device_action_improvement,
			mfds_device_action_monitoring,
			mfds_device_action_notification,
			mfds_device_action_label_change,
			mfds_device_action_other,
			start_date,
			start_date_null_flavor,
			end_date,
			end_date_null_flavor,
			duration_value,
			duration_unit,
			outcome,
			medical_confirmation,
			country_code,
		});
	}

	Ok(imports)
}

/// e2b:E.i.1.1a
fn read_e_i_1_1a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, EReactionPaths::PRIMARY_TEXT),
		"primarySourceReaction",
		input_contracts::generated::e::e_i_1_1a,
	)
}

/// e2b:E.i.1.1b
fn read_e_i_1_1b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, EReactionPaths::TRANSLATION_TEXT),
		"primarySourceReactionTranslation",
		input_contracts::generated::e::e_i_1_2,
	)
}

/// e2b:E.i.1.2
fn read_e_i_1_2(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, EReactionPaths::PRIMARY_LANG),
		"reactionLanguage",
		input_contracts::generated::e::e_i_1_1b,
	)
}

/// e2b:E.i.2.1a
fn read_e_i_2_1a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, EReactionPaths::MEDDRA_VERSION),
		"reactionMeddraVersionLLT",
		input_contracts::generated::e::e_i_2_1a,
	)
}

/// e2b:E.i.2.1b
fn read_e_i_2_1b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, EReactionPaths::MEDDRA_CODE),
		"reactionMeddraCodeLLT",
		input_contracts::generated::e::e_i_2_1b,
	)
}

/// e2b:E.i.3.1
fn read_e_i_3_1(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let code = first_attr(xpath, node, EReactionPaths::TERM_HIGHLIGHT_CODE);
	import_constraint::string(
		"termHighlighted",
		code.as_deref(),
		None,
		input_contracts::generated::e::e_i_3_1,
	)?;
	Ok(code)
}

/// e2b:E.i.serious
fn read_e_i_serious(criteria: [Option<bool>; 6]) -> Option<bool> {
	let values = criteria.into_iter().flatten().collect::<Vec<_>>();
	(!values.is_empty()).then_some(values.into_iter().any(|value| value))
}

fn read_seriousness(
	xpath: &mut Context,
	node: &Node,
	value_path: &str,
	null_flavor_path: &str,
	field: &str,
	_null_field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<(Option<bool>, Option<String>)> {
	let value = parse_bool_literal(first_attr(xpath, node, value_path).as_deref());
	let null_flavor = first_attr(xpath, node, null_flavor_path);
	import_constraint::boolean(field, value, null_flavor.as_deref(), check)?;
	Ok((value, null_flavor))
}

/// e2b:E.i.3.2a
fn read_e_i_3_2a(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<bool>, Option<String>)> {
	read_seriousness(
		xpath,
		node,
		EReactionPaths::CRITERIA_DEATH,
		EReactionPaths::CRITERIA_DEATH_NULL_FLAVOR,
		"seriousness.criteriaResultsInDeath",
		"seriousness.criteriaResultsInDeathNullFlavor",
		input_contracts::generated::e::e_i_3_2a,
	)
}

/// e2b:E.i.3.2b
fn read_e_i_3_2b(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<bool>, Option<String>)> {
	read_seriousness(
		xpath,
		node,
		EReactionPaths::CRITERIA_LIFE_THREATENING,
		EReactionPaths::CRITERIA_LIFE_THREATENING_NULL_FLAVOR,
		"seriousness.criteriaLifeThreatening",
		"seriousness.criteriaLifeThreateningNullFlavor",
		input_contracts::generated::e::e_i_3_2b,
	)
}

/// e2b:E.i.3.2c
fn read_e_i_3_2c(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<bool>, Option<String>)> {
	read_seriousness(
		xpath,
		node,
		EReactionPaths::CRITERIA_HOSPITALIZATION,
		EReactionPaths::CRITERIA_HOSPITALIZATION_NULL_FLAVOR,
		"seriousness.criteriaHospitalization",
		"seriousness.criteriaHospitalizationNullFlavor",
		input_contracts::generated::e::e_i_3_2c,
	)
}

/// e2b:E.i.3.2d
fn read_e_i_3_2d(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<bool>, Option<String>)> {
	read_seriousness(
		xpath,
		node,
		EReactionPaths::CRITERIA_DISABLING,
		EReactionPaths::CRITERIA_DISABLING_NULL_FLAVOR,
		"seriousness.criteriaDisabling",
		"seriousness.criteriaDisablingNullFlavor",
		input_contracts::generated::e::e_i_3_2d,
	)
}

/// e2b:E.i.3.2e
fn read_e_i_3_2e(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<bool>, Option<String>)> {
	read_seriousness(
		xpath,
		node,
		EReactionPaths::CRITERIA_CONGENITAL,
		EReactionPaths::CRITERIA_CONGENITAL_NULL_FLAVOR,
		"seriousness.criteriaCongenitalAnomaly",
		"seriousness.criteriaCongenitalAnomalyNullFlavor",
		input_contracts::generated::e::e_i_3_2e,
	)
}

/// e2b:E.i.3.2f
fn read_e_i_3_2f(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<bool>, Option<String>)> {
	read_seriousness(
		xpath,
		node,
		EReactionPaths::CRITERIA_OTHER,
		EReactionPaths::CRITERIA_OTHER_NULL_FLAVOR,
		"seriousness.criteriaOtherMedicallyImportant",
		"seriousness.criteriaOtherMedicallyImportantNullFlavor",
		input_contracts::generated::e::e_i_3_2f,
	)
}

/// e2b:FDA.E.i.3.2h
fn read_fda_e_i_3_2h(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<bool>, Option<String>)> {
	let raw = first_attr(xpath, node, EReactionPaths::REQUIRED_INTERVENTION);
	let value = raw.as_deref().and_then(normalize_xml_bool);
	let null_flavor = first_attr(
		xpath,
		node,
		EReactionPaths::REQUIRED_INTERVENTION_NULL_FLAVOR,
	);
	import_constraint::boolean(
		"requiredIntervention",
		value,
		null_flavor.as_deref(),
		input_contracts::generated::e::fda_e_i_3_2h,
	)?;
	import_constraint::string(
		"requiredInterventionNullFlavor",
		None,
		None,
		input_contracts::generated::e::fda_e_i_3_2h,
	)?;
	Ok((value, null_flavor))
}

fn read_date(
	xpath: &mut Context,
	node: &Node,
	value_path: &str,
	null_flavor_path: &str,
	field: &str,
	_null_field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<(Option<String>, Option<String>)> {
	let raw = first_attr(xpath, node, value_path);
	let null_flavor = first_attr(xpath, node, null_flavor_path);
	import_constraint::string(field, raw.as_deref(), null_flavor.as_deref(), check)?;
	Ok((raw, null_flavor))
}

/// e2b:E.i.4
fn read_e_i_4(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	read_date(
		xpath,
		node,
		EReactionPaths::START_DATE,
		EReactionPaths::START_DATE_NULL_FLAVOR,
		"reactionStartDate",
		"reactionStartDateNullFlavor",
		input_contracts::generated::e::e_i_4,
	)
}

/// e2b:E.i.5
fn read_e_i_5(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	read_date(
		xpath,
		node,
		EReactionPaths::END_DATE,
		EReactionPaths::END_DATE_NULL_FLAVOR,
		"reactionEndDate",
		"reactionEndDateNullFlavor",
		input_contracts::generated::e::e_i_5,
	)
}

/// e2b:E.i.6a
fn read_e_i_6a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let raw = first_attr(xpath, node, EReactionPaths::DURATION_VALUE);
	let raw = raw
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty());
	import_constraint::string(
		"reactionDuration.value",
		raw.as_deref(),
		None,
		input_contracts::generated::e::e_i_6a,
	)?;
	Ok(raw)
}

/// e2b:E.i.6b
fn read_e_i_6b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let unit = input_string(
		first_attr(xpath, node, EReactionPaths::DURATION_UNIT),
		"reactionDuration.unit",
		input_contracts::generated::e::e_i_6b,
	)?;
	let Some(unit) = unit else {
		return Ok(None);
	};
	Ok(Some(
		crate::mapping::fda::e_reaction::reaction_duration_unit_from_ucum(&unit)
			.unwrap_or(&unit)
			.to_string(),
	))
}

/// e2b:E.i.7
fn read_e_i_7(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, EReactionPaths::OUTCOME_CODE),
		"reactionOutcome",
		input_contracts::generated::e::e_i_7,
	)
}

/// e2b:E.i.8
fn read_e_i_8(xpath: &mut Context, node: &Node) -> Result<Option<bool>> {
	let value = parse_bool_literal(
		first_attr(xpath, node, EReactionPaths::MEDICAL_CONFIRMATION).as_deref(),
	);
	import_constraint::boolean(
		"medicalConfirmation",
		value,
		None,
		input_contracts::generated::e::e_i_8,
	)?;
	Ok(value)
}

/// e2b:E.i.9
fn read_e_i_9(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	Ok(input_string(
		first_attr(xpath, node, EReactionPaths::COUNTRY_CODE),
		"reactionCountry",
		input_contracts::generated::e::e_i_9,
	)?
	.map(|value| value.to_ascii_uppercase()))
}

/// e2b:E.local.expectedness
fn read_e_local_expectedness(xpath: &mut Context, node: &Node) -> Option<String> {
	extension_code(xpath, node, "AE_EXPECTEDNESS")
}

/// e2b:E.local.severity
fn read_e_local_severity(xpath: &mut Context, node: &Node) -> Option<String> {
	extension_code(xpath, node, "AE_SEVERITY")
}

/// e2b:E.i.KR.device.aeClassification
fn read_e_i_kr_device_ae_classification(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	extension_code(xpath, node, "KR_DVC_AECL")
}

/// e2b:E.i.KR.device.aeOutcome
fn read_e_i_kr_device_ae_outcome(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	extension_code(xpath, node, "KR_DVC_AEOUT")
}

/// e2b:E.i.KR.device.causeMedicalDevice
fn read_e_i_kr_device_cause_medical_device(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_CC_MD")
}

/// e2b:E.i.KR.device.causeProcedureIssue
fn read_e_i_kr_device_cause_procedure_issue(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_CC_PI")
}

/// e2b:E.i.KR.device.causePatientCondition
fn read_e_i_kr_device_cause_patient_condition(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_CC_PC")
}

/// e2b:E.i.KR.device.causeUnableToAssess
fn read_e_i_kr_device_cause_unable_to_assess(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_CC_UA")
}

/// e2b:E.i.KR.device.causeOther
fn read_e_i_kr_device_cause_other(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	extension_text(xpath, node, "KR_DVC_CC_OTH")
}

/// e2b:E.i.KR.device.actionReason
fn read_e_i_kr_device_action_reason(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	extension_text(xpath, node, "KR_DVC_ACT_RSN")
}

/// e2b:E.i.KR.device.actionRecall
fn read_e_i_kr_device_action_recall(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_ACT_RC")
}

/// e2b:E.i.KR.device.actionRepair
fn read_e_i_kr_device_action_repair(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_ACT_RP")
}

/// e2b:E.i.KR.device.actionInspection
fn read_e_i_kr_device_action_inspection(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_ACT_INSP")
}

/// e2b:E.i.KR.device.actionReplacement
fn read_e_i_kr_device_action_replacement(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_ACT_REPL")
}

/// e2b:E.i.KR.device.actionImprovement
fn read_e_i_kr_device_action_improvement(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_ACT_IMP")
}

/// e2b:E.i.KR.device.actionMonitoring
fn read_e_i_kr_device_action_monitoring(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_ACT_MON")
}

/// e2b:E.i.KR.device.actionNotification
fn read_e_i_kr_device_action_notification(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_ACT_NTF")
}

/// e2b:E.i.KR.device.actionLabelChange
fn read_e_i_kr_device_action_label_change(
	xpath: &mut Context,
	node: &Node,
) -> Option<bool> {
	extension_bool(xpath, node, "KR_DVC_ACT_CAS")
}

/// e2b:E.i.KR.device.actionOther
fn read_e_i_kr_device_action_other(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	extension_text(xpath, node, "KR_DVC_ACT_OTH")
}

fn extension_bool(xpath: &mut Context, node: &Node, code: &str) -> Option<bool> {
	parse_bool_literal(extension_value_attr(xpath, node, code, "value").as_deref())
}

fn extension_code(xpath: &mut Context, node: &Node, code: &str) -> Option<String> {
	extension_value_attr(xpath, node, code, "code")
}

fn extension_text(xpath: &mut Context, node: &Node, code: &str) -> Option<String> {
	let expr = format!(
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='{code}']]/hl7:value"
	);
	first_text(xpath, node, &expr)
}

fn extension_value_attr(
	xpath: &mut Context,
	node: &Node,
	code: &str,
	attr: &str,
) -> Option<String> {
	let expr = format!(
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='{code}']]/hl7:value/@{attr}"
	);
	first_attr(xpath, node, &expr)
}

pub(crate) async fn import_section_e(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<ImportIdMap> {
	let mut map = ImportIdMap::default();
	for (idx, reaction) in parse_e_reactions(xml)?.into_iter().enumerate() {
		let rec_id = ReactionBmc::create(
			ctx,
			mm,
			ReactionForCreate {
				case_id,
				sequence_number: (idx + 1) as i32,
				primary_source_reaction: reaction.primary_source_reaction,
				primary_source_reaction_translation: reaction
					.primary_source_reaction_translation,
				reaction_language: reaction.reaction_language,
				reaction_meddra_code: reaction.reaction_meddra_code,
				reaction_meddra_version: reaction.reaction_meddra_version,
				term_highlighted: reaction.term_highlighted,
				serious: reaction.serious,
				criteria_death: reaction.criteria_death,
				criteria_death_null_flavor: reaction.criteria_death_null_flavor,
				criteria_life_threatening: reaction.criteria_life_threatening,
				criteria_life_threatening_null_flavor: reaction
					.criteria_life_threatening_null_flavor,
				criteria_hospitalization: reaction.criteria_hospitalization,
				criteria_hospitalization_null_flavor: reaction
					.criteria_hospitalization_null_flavor,
				criteria_disabling: reaction.criteria_disabling,
				criteria_disabling_null_flavor: reaction
					.criteria_disabling_null_flavor,
				criteria_congenital_anomaly: reaction.criteria_congenital_anomaly,
				criteria_congenital_anomaly_null_flavor: reaction
					.criteria_congenital_anomaly_null_flavor,
				criteria_other_medically_important: reaction
					.criteria_other_medically_important,
				criteria_other_medically_important_null_flavor: reaction
					.criteria_other_medically_important_null_flavor,
				required_intervention: reaction.required_intervention,
				required_intervention_null_flavor: reaction
					.required_intervention_null_flavor,
				expectedness: reaction.expectedness,
				severity: reaction.severity,
				mfds_device_ae_classification: reaction
					.mfds_device_ae_classification,
				mfds_device_ae_outcome: reaction.mfds_device_ae_outcome,
				mfds_device_cause_medical_device: reaction
					.mfds_device_cause_medical_device,
				mfds_device_cause_procedure_issue: reaction
					.mfds_device_cause_procedure_issue,
				mfds_device_cause_patient_condition: reaction
					.mfds_device_cause_patient_condition,
				mfds_device_cause_unable_to_assess: reaction
					.mfds_device_cause_unable_to_assess,
				mfds_device_cause_other: reaction.mfds_device_cause_other,
				mfds_device_action_reason: reaction.mfds_device_action_reason,
				mfds_device_action_recall: reaction.mfds_device_action_recall,
				mfds_device_action_repair: reaction.mfds_device_action_repair,
				mfds_device_action_inspection: reaction
					.mfds_device_action_inspection,
				mfds_device_action_replacement: reaction
					.mfds_device_action_replacement,
				mfds_device_action_improvement: reaction
					.mfds_device_action_improvement,
				mfds_device_action_monitoring: reaction
					.mfds_device_action_monitoring,
				mfds_device_action_notification: reaction
					.mfds_device_action_notification,
				mfds_device_action_label_change: reaction
					.mfds_device_action_label_change,
				mfds_device_action_other: reaction.mfds_device_action_other,
				start_date: reaction.start_date,
				start_date_null_flavor: reaction.start_date_null_flavor,
				end_date: reaction.end_date,
				end_date_null_flavor: reaction.end_date_null_flavor,
				duration_value: reaction.duration_value,
				duration_unit: reaction.duration_unit,
				outcome: reaction.outcome,
				medical_confirmation: reaction.medical_confirmation,
				country_code: reaction.country_code,
				deleted: Some(false),
			},
		)
		.await?;
		if let Some(xml_id) = reaction.xml_id {
			map.insert_xml_id(xml_id, rec_id);
		}
		map.push_sequence(rec_id);
	}
	Ok(map)
}

fn normalize_xml_bool(value: &str) -> Option<bool> {
	match value.trim().to_ascii_lowercase().as_str() {
		"true" | "1" => Some(true),
		"false" | "0" | "2" => Some(false),
		_ => None,
	}
}

fn first_attr(xpath: &mut Context, node: &Node, expr: &str) -> Option<String> {
	xpath
		.findvalues(expr, Some(node))
		.ok()?
		.into_iter()
		.find(|v| !v.trim().is_empty())
}

fn first_text(xpath: &mut Context, node: &Node, expr: &str) -> Option<String> {
	let nodes = xpath.findnodes(expr, Some(node)).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

fn input_string(
	value: Option<String>,
	field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<Option<String>> {
	import_constraint::string(field, value.as_deref(), None, check)?;
	Ok(value)
}

#[cfg(test)]
mod split_null_flavor_tests {
	use super::parse_e_reactions;

	fn reaction_with_duration_unit(unit: &str) -> String {
		format!(
			r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><subjectOf2><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><effectiveTime xsi:type="SXPR_TS"><comp xsi:type="IVL_TS" operator="A"><width value="1" unit="{unit}"/></comp></effectiveTime><value xsi:type="CE"><originalText>Reaction</originalText></value></observation></subjectOf2></MCCI_IN200100UV01>"#
		)
	}

	#[test]
	fn imports_required_intervention_boolean_null_flavor_into_companion() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><subjectOf2><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value xsi:type="CE"><originalText>Reaction</originalText></value><outboundRelationship2><observation><code code="7"/><value xsi:type="BL" nullFlavor="NI"/></observation></outboundRelationship2></observation></subjectOf2></MCCI_IN200100UV01>"#;
		let reactions = parse_e_reactions(xml).expect("parse");
		assert_eq!(reactions[0].required_intervention, None);
		assert_eq!(
			reactions[0].required_intervention_null_flavor.as_deref(),
			Some("NI")
		);
	}

	#[test]
	fn imports_ucum_duration_units_and_preserves_unknown_units() {
		for (ucum, stored) in [
			("10.a", "800"),
			("a", "801"),
			("mo", "802"),
			("wk", "803"),
			("d", "804"),
			("h", "805"),
		] {
			let xml = reaction_with_duration_unit(ucum);
			let reactions = parse_e_reactions(xml.as_bytes())
				.expect("supported UCUM reaction duration unit");
			assert_eq!(reactions[0].duration_unit.as_deref(), Some(stored));
			assert_eq!(reactions[0].duration_value.as_deref(), Some("1"));
		}

		let xml = reaction_with_duration_unit("min");
		let reactions = parse_e_reactions(xml.as_bytes()).expect("parse");
		assert_eq!(reactions[0].duration_unit.as_deref(), Some("min"));
	}

	#[test]
	fn imports_reaction_dates_from_mfds_sxpr_components() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><subjectOf2><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><effectiveTime xsi:type="SXPR_TS"><comp xsi:type="IVL_TS"><low value="20081009"/><high value="20081201"/></comp><comp xsi:type="IVL_TS" operator="A"><width value="54" unit="d"/></comp></effectiveTime><value xsi:type="CE"><originalText>Reaction</originalText></value></observation></subjectOf2></MCCI_IN200100UV01>"#;
		let reaction = parse_e_reactions(xml).expect("parse").remove(0);

		assert_eq!(reaction.start_date.as_deref(), Some("20081009"));
		assert_eq!(reaction.end_date.as_deref(), Some("20081201"));
		assert_eq!(reaction.duration_value.as_deref(), Some("54"));
		assert_eq!(reaction.duration_unit.as_deref(), Some("804"));
	}

	#[test]
	fn imports_duration_from_the_official_first_sxpr_component() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><subjectOf2><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><effectiveTime xsi:type="SXPR_TS"><comp xsi:type="IVL_TS"><low value="20030511"/><width value="1.00" unit="wk"/></comp><comp xsi:type="IVL_TS" operator="A"><high value="20030518"/></comp></effectiveTime><value xsi:type="CE"><originalText>Reaction</originalText></value></observation></subjectOf2></MCCI_IN200100UV01>"#;
		let reaction = parse_e_reactions(xml).expect("parse").remove(0);

		assert_eq!(reaction.duration_value.as_deref(), Some("1.00"));
		assert_eq!(reaction.duration_unit.as_deref(), Some("803"));
	}

	#[test]
	fn imports_duration_from_direct_effective_time_width() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><subjectOf2><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><effectiveTime xsi:type="IVL_TS"><width value="24" unit="h"/></effectiveTime><value xsi:type="CE"><originalText>Reaction</originalText></value></observation></subjectOf2></MCCI_IN200100UV01>"#;
		let reaction = parse_e_reactions(xml).expect("parse").remove(0);

		assert_eq!(reaction.duration_value.as_deref(), Some("24"));
		assert_eq!(reaction.duration_unit.as_deref(), Some("805"));
	}

	#[test]
	fn rejects_non_numeric_duration_values_before_db() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><subjectOf2><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><effectiveTime xsi:type="SXPR_TS"><comp xsi:type="IVL_TS"><width value="54x" unit="d"/></comp></effectiveTime><value xsi:type="CE"><originalText>Reaction</originalText></value></observation></subjectOf2></MCCI_IN200100UV01>"#;
		let error =
			parse_e_reactions(xml).expect_err("invalid duration must fail import");

		assert!(error.to_string().contains("reactionDuration.value"));
	}

	#[test]
	fn preserves_partial_and_offset_reaction_ts() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><subjectOf2><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><effectiveTime xsi:type="IVL_TS"><low value="202206"/><high value="200509211242-08"/></effectiveTime><value xsi:type="CE"><originalText>Reaction</originalText></value></observation></subjectOf2></MCCI_IN200100UV01>"#;
		let reaction = parse_e_reactions(xml).expect("parse").remove(0);
		assert_eq!(reaction.start_date.as_deref(), Some("202206"));
		assert_eq!(reaction.end_date.as_deref(), Some("200509211242-08"));
	}
}
