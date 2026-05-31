// Section E importer (Reaction/Event) - FDA mapping.

use crate::xml::error::Error;
use crate::xml::mapping::fda::e_reaction::EReactionPaths;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Node;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use time::Month;

#[derive(Debug)]
pub struct EReactionImport {
	pub xml_id: Option<Uuid>,
	pub primary_source_reaction: String,
	pub primary_source_reaction_translation: Option<String>,
	pub reaction_language: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub term_highlighted: Option<bool>,
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
	pub required_intervention: Option<String>,
	pub included_in_ema_ime_list: Option<bool>,
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
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<Decimal>,
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
	for (idx, node) in nodes.into_iter().enumerate() {
		let xml_id = parse_uuid_opt(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::XML_ID_ROOT,
		));
		let translation_text =
			first_text(&mut xpath, &node, EReactionPaths::TRANSLATION_TEXT);
		let primary = first_text(&mut xpath, &node, EReactionPaths::PRIMARY_TEXT)
			.or_else(|| first_text(&mut xpath, &node, EReactionPaths::PRIMARY_TEXT_ALT))
			.or_else(|| translation_text.clone())
			.unwrap_or_else(|| {
				eprintln!(
					"[import_e2b_xml] reactions[{idx}] missing E.i.1.1a text; importing empty primary_source_reaction for downstream validation"
				);
				String::new()
			});

		let reaction_meddra_version = clamp_str(
			first_attr(&mut xpath, &node, EReactionPaths::MEDDRA_VERSION),
			10,
		);
		let reaction_meddra_code =
			first_attr(&mut xpath, &node, EReactionPaths::MEDDRA_CODE);
		let reaction_language = normalize_lang2(
			first_attr(&mut xpath, &node, EReactionPaths::PRIMARY_LANG),
			"reactions.reaction_language",
		);

		let term_code =
			first_attr(&mut xpath, &node, EReactionPaths::TERM_HIGHLIGHT_CODE);
		let term_highlighted = term_code.as_deref().and_then(|v| match v {
			"1" | "3" => Some(true),
			"2" | "4" => Some(false),
			_ => None,
		});
		let serious_from_term = term_code.as_deref().and_then(|v| match v {
			"3" | "4" => Some(true),
			"1" | "2" => Some(false),
			_ => None,
		});
		let (criteria_death, criteria_death_null_flavor) =
			parse_bool_with_null_flavor(
				first_attr(&mut xpath, &node, EReactionPaths::CRITERIA_DEATH),
				first_attr(
					&mut xpath,
					&node,
					EReactionPaths::CRITERIA_DEATH_NULL_FLAVOR,
				),
			);
		let (criteria_life_threatening, criteria_life_threatening_null_flavor) =
			parse_bool_with_null_flavor(
				first_attr(
					&mut xpath,
					&node,
					EReactionPaths::CRITERIA_LIFE_THREATENING,
				),
				first_attr(
					&mut xpath,
					&node,
					EReactionPaths::CRITERIA_LIFE_THREATENING_NULL_FLAVOR,
				),
			);
		let (criteria_hospitalization, criteria_hospitalization_null_flavor) =
			parse_bool_with_null_flavor(
				first_attr(
					&mut xpath,
					&node,
					EReactionPaths::CRITERIA_HOSPITALIZATION,
				),
				first_attr(
					&mut xpath,
					&node,
					EReactionPaths::CRITERIA_HOSPITALIZATION_NULL_FLAVOR,
				),
			);
		let (criteria_disabling, criteria_disabling_null_flavor) =
			parse_bool_with_null_flavor(
				first_attr(&mut xpath, &node, EReactionPaths::CRITERIA_DISABLING),
				first_attr(
					&mut xpath,
					&node,
					EReactionPaths::CRITERIA_DISABLING_NULL_FLAVOR,
				),
			);
		let (criteria_congenital_anomaly, criteria_congenital_anomaly_null_flavor) =
			parse_bool_with_null_flavor(
				first_attr(&mut xpath, &node, EReactionPaths::CRITERIA_CONGENITAL),
				first_attr(
					&mut xpath,
					&node,
					EReactionPaths::CRITERIA_CONGENITAL_NULL_FLAVOR,
				),
			);
		let (
			criteria_other_medically_important,
			criteria_other_medically_important_null_flavor,
		) = parse_bool_with_null_flavor(
			first_attr(&mut xpath, &node, EReactionPaths::CRITERIA_OTHER),
			first_attr(
				&mut xpath,
				&node,
				EReactionPaths::CRITERIA_OTHER_NULL_FLAVOR,
			),
		);
		let criteria_any_true = [
			criteria_death,
			criteria_life_threatening,
			criteria_hospitalization,
			criteria_disabling,
			criteria_congenital_anomaly,
			criteria_other_medically_important,
		]
		.into_iter()
		.flatten()
		.any(|v| v);
		let serious = if criteria_any_true {
			Some(true)
		} else {
			serious_from_term
		};

		let required_intervention = clamp_str(
			first_attr(&mut xpath, &node, EReactionPaths::REQUIRED_INTERVENTION),
			10,
		);
		let included_in_ema_ime_list =
			extension_bool(&mut xpath, &node, "AE_IME_LIST");
		let expectedness =
			clamp_str(extension_code(&mut xpath, &node, "AE_EXPECTEDNESS"), 1);
		let severity =
			clamp_str(extension_code(&mut xpath, &node, "AE_SEVERITY"), 20);
		let mfds_device_ae_classification =
			clamp_str(extension_code(&mut xpath, &node, "KR_DVC_AECL"), 1);
		let mfds_device_ae_outcome =
			clamp_str(extension_code(&mut xpath, &node, "KR_DVC_AEOUT"), 2);
		let mfds_device_cause_medical_device =
			extension_bool(&mut xpath, &node, "KR_DVC_CC_MD");
		let mfds_device_cause_procedure_issue =
			extension_bool(&mut xpath, &node, "KR_DVC_CC_PI");
		let mfds_device_cause_patient_condition =
			extension_bool(&mut xpath, &node, "KR_DVC_CC_PC");
		let mfds_device_cause_unable_to_assess =
			extension_bool(&mut xpath, &node, "KR_DVC_CC_UA");
		let mfds_device_cause_other =
			clamp_str(extension_text(&mut xpath, &node, "KR_DVC_CC_OTH"), 20000);
		let mfds_device_action_reason =
			clamp_str(extension_text(&mut xpath, &node, "KR_DVC_ACT_RSN"), 20000);
		let mfds_device_action_recall =
			extension_bool(&mut xpath, &node, "KR_DVC_ACT_RC");
		let mfds_device_action_repair =
			extension_bool(&mut xpath, &node, "KR_DVC_ACT_RP");
		let mfds_device_action_inspection =
			extension_bool(&mut xpath, &node, "KR_DVC_ACT_INSP");
		let mfds_device_action_replacement =
			extension_bool(&mut xpath, &node, "KR_DVC_ACT_REPL");
		let mfds_device_action_improvement =
			extension_bool(&mut xpath, &node, "KR_DVC_ACT_IMP");
		let mfds_device_action_monitoring =
			extension_bool(&mut xpath, &node, "KR_DVC_ACT_MON");
		let mfds_device_action_notification =
			extension_bool(&mut xpath, &node, "KR_DVC_ACT_NTF");
		let mfds_device_action_label_change =
			extension_bool(&mut xpath, &node, "KR_DVC_ACT_CAS");
		let mfds_device_action_other =
			clamp_str(extension_text(&mut xpath, &node, "KR_DVC_ACT_OTH"), 20000);
		let start_date = first_attr(&mut xpath, &node, EReactionPaths::START_DATE)
			.or_else(|| {
				first_attr(&mut xpath, &node, EReactionPaths::START_DATE_FALLBACK)
			})
			.and_then(parse_date);
		let start_date_null_flavor =
			first_attr(&mut xpath, &node, EReactionPaths::START_DATE_NULL_FLAVOR)
				.or_else(|| {
					first_attr(
						&mut xpath,
						&node,
						EReactionPaths::START_DATE_NULL_FLAVOR_FALLBACK,
					)
				});
		let end_date = first_attr(&mut xpath, &node, EReactionPaths::END_DATE)
			.or_else(|| {
				first_attr(&mut xpath, &node, EReactionPaths::END_DATE_FALLBACK)
			})
			.and_then(parse_date);
		let end_date_null_flavor =
			first_attr(&mut xpath, &node, EReactionPaths::END_DATE_NULL_FLAVOR)
				.or_else(|| {
					first_attr(
						&mut xpath,
						&node,
						EReactionPaths::END_DATE_NULL_FLAVOR_FALLBACK,
					)
				});
		let duration_value =
			first_attr(&mut xpath, &node, EReactionPaths::DURATION_VALUE)
				.and_then(|v| v.parse::<Decimal>().ok());
		let duration_unit = normalize_code3(
			first_attr(&mut xpath, &node, EReactionPaths::DURATION_UNIT),
			"reactions.duration_unit",
		);
		let outcome = first_attr(&mut xpath, &node, EReactionPaths::OUTCOME_CODE);
		let medical_confirmation = parse_bool_value(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::MEDICAL_CONFIRMATION,
		));
		let country_code = normalize_iso2(
			first_attr(&mut xpath, &node, EReactionPaths::COUNTRY_CODE),
			"reactions.country_code",
		);

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
			included_in_ema_ime_list,
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

fn extension_bool(xpath: &mut Context, node: &Node, code: &str) -> Option<bool> {
	parse_bool_value(extension_value_attr(xpath, node, code, "value"))
}

fn extension_code(xpath: &mut Context, node: &Node, code: &str) -> Option<String> {
	extension_value_attr(xpath, node, code, "code")
		.or_else(|| extension_value_attr(xpath, node, code, "value"))
		.or_else(|| extension_text(xpath, node, code))
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

fn parse_bool_value(value: Option<String>) -> Option<bool> {
	let val = value?;
	match val.to_ascii_lowercase().as_str() {
		"true" | "1" => Some(true),
		"false" | "0" => Some(false),
		_ => None,
	}
}

fn parse_bool_with_null_flavor(
	value: Option<String>,
	null_flavor: Option<String>,
) -> (Option<bool>, Option<String>) {
	(parse_bool_value(value), null_flavor)
}

fn clamp_str(value: Option<String>, max: usize) -> Option<String> {
	match value {
		Some(v) if v.len() > max => Some(v.chars().take(max).collect()),
		other => other,
	}
}

fn parse_uuid_opt(value: Option<String>) -> Option<Uuid> {
	let value = value?.trim().to_string();
	if value.is_empty() {
		return None;
	}
	Uuid::parse_str(&value).ok()
}

fn normalize_iso2(value: Option<String>, _field: &str) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	let upper = v.to_ascii_uppercase();
	if len == 2 && upper.chars().all(|c| c.is_ascii_uppercase()) {
		Some(upper)
	} else {
		None
	}
}

fn normalize_lang2(value: Option<String>, _field: &str) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	let lower = v.to_ascii_lowercase();
	if len == 2 && lower.chars().all(|c| c.is_ascii_lowercase()) {
		Some(lower)
	} else {
		None
	}
}

fn normalize_code3(value: Option<String>, _field: &str) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	if (1..=3).contains(&len) && v.chars().all(|c| c.is_ascii_alphanumeric()) {
		Some(v)
	} else {
		None
	}
}

fn parse_date(value: String) -> Option<Date> {
	let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
	if digits.len() < 8 {
		return None;
	}
	let y: i32 = digits[0..4].parse().ok()?;
	let m: u8 = digits[4..6].parse().ok()?;
	let d: u8 = digits[6..8].parse().ok()?;
	let month = Month::try_from(m).ok()?;
	Date::from_calendar_date(y, month, d).ok()
}
