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
	pub criteria_life_threatening: Option<bool>,
	pub criteria_hospitalization: Option<bool>,
	pub criteria_disabling: Option<bool>,
	pub criteria_congenital_anomaly: Option<bool>,
	pub criteria_other_medically_important: Option<bool>,
	pub required_intervention: Option<String>,
	pub start_date: Option<Date>,
	pub end_date: Option<Date>,
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
			.or_else(|| translation_text.clone())
			.ok_or_else(|| Error::InvalidXml {
				message: format!(
					"ICH.E.i.1.1a.REQUIRED: reaction text missing for reaction index {}",
					idx + 1
				),
				line: None,
				column: None,
			})?;

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
		let criteria_death = parse_bool_value(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::CRITERIA_DEATH,
		));
		let criteria_life_threatening = parse_bool_value(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::CRITERIA_LIFE_THREATENING,
		));
		let criteria_hospitalization = parse_bool_value(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::CRITERIA_HOSPITALIZATION,
		));
		let criteria_disabling = parse_bool_value(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::CRITERIA_DISABLING,
		));
		let criteria_congenital_anomaly = parse_bool_value(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::CRITERIA_CONGENITAL,
		));
		let criteria_other_medically_important = parse_bool_value(first_attr(
			&mut xpath,
			&node,
			EReactionPaths::CRITERIA_OTHER,
		));
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
		let start_date = first_attr(&mut xpath, &node, EReactionPaths::START_DATE)
			.or_else(|| {
				first_attr(&mut xpath, &node, EReactionPaths::START_DATE_FALLBACK)
			})
			.and_then(parse_date);
		let end_date = first_attr(&mut xpath, &node, EReactionPaths::END_DATE)
			.or_else(|| {
				first_attr(&mut xpath, &node, EReactionPaths::END_DATE_FALLBACK)
			})
			.and_then(parse_date);
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
			criteria_life_threatening,
			criteria_hospitalization,
			criteria_disabling,
			criteria_congenital_anomaly,
			criteria_other_medically_important,
			required_intervention,
			start_date,
			end_date,
			duration_value,
			duration_unit,
			outcome,
			medical_confirmation,
			country_code,
		});
	}

	Ok(imports)
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
