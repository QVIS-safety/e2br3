// Section D importer (Patient) - FDA mapping.

use crate::xml::error::Error;
use crate::xml::mapping::fda::d_patient::DPatientPaths;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use sqlx::types::time::Date;
use time::Month;

#[derive(Debug)]
pub struct DPatientImport {
	pub patient_initials: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	pub birth_date: Option<Date>,
	pub birth_date_null_flavor: Option<String>,
	pub sex: Option<String>,
	pub sex_null_flavor: Option<String>,
	pub age_at_time_of_onset: Option<Decimal>,
	pub age_at_time_of_onset_null_flavor: Option<String>,
	pub age_unit: Option<String>,
	pub gestation_period: Option<Decimal>,
	pub gestation_period_unit: Option<String>,
	pub age_group: Option<String>,
	pub weight_kg: Option<Decimal>,
	pub height_cm: Option<Decimal>,
	pub race_code: Option<String>,
	pub race_code_null_flavor: Option<String>,
	pub ethnicity_code: Option<String>,
	pub ethnicity_code_null_flavor: Option<String>,
	pub last_menstrual_period_date: Option<Date>,
	pub last_menstrual_period_date_null_flavor: Option<String>,
	pub medical_history_text: Option<String>,
	pub medical_history_text_null_flavor: Option<String>,
	pub concomitant_therapy: Option<bool>,
}

/// Parse Section D values using FDA/ICH mapping paths.
pub fn parse_d_patient(xml: &[u8]) -> Result<Option<DPatientImport>> {
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

	let patient_name = first_text_root(&mut xpath, DPatientPaths::PATIENT_NAME);
	let patient_initials = patient_name;
	let patient_initials_null_flavor =
		first_value_root(&mut xpath, DPatientPaths::PATIENT_NAME_NULL_FLAVOR);

	let sex =
		normalize_sex_code(first_value_root(&mut xpath, DPatientPaths::SEX_CODE));
	let sex_null_flavor =
		first_value_root(&mut xpath, DPatientPaths::SEX_NULL_FLAVOR);
	let birth_date =
		first_value_root(&mut xpath, DPatientPaths::BIRTH_DATE).and_then(parse_date);
	let birth_date_null_flavor =
		first_value_root(&mut xpath, DPatientPaths::BIRTH_DATE_NULL_FLAVOR);
	let age_at_time_of_onset =
		first_value_root(&mut xpath, DPatientPaths::AGE_VALUE)
			.and_then(|v| v.parse::<Decimal>().ok());
	let age_at_time_of_onset_null_flavor =
		first_value_root(&mut xpath, DPatientPaths::AGE_NULL_FLAVOR);
	let age_unit = normalize_code3(
		first_value_root(&mut xpath, DPatientPaths::AGE_UNIT),
		"patient_information.age_unit",
	);
	let gestation_period =
		first_value_root(&mut xpath, DPatientPaths::GESTATION_VALUE)
			.and_then(|v| v.parse::<Decimal>().ok());
	let gestation_period_unit = normalize_code3(
		first_value_root(&mut xpath, DPatientPaths::GESTATION_UNIT),
		"patient_information.gestation_period_unit",
	);
	let age_group = normalize_code(
		first_value_root(&mut xpath, DPatientPaths::AGE_GROUP_CODE),
		&["1", "2", "3", "4", "5", "6"],
		"patient_information.age_group",
	);
	let weight_kg = first_value_root(&mut xpath, DPatientPaths::WEIGHT_VALUE)
		.and_then(|v| v.parse::<Decimal>().ok());
	let height_cm = first_value_root(&mut xpath, DPatientPaths::HEIGHT_VALUE)
		.and_then(|v| v.parse::<Decimal>().ok());
	let last_menstrual_period_date =
		first_value_root(&mut xpath, DPatientPaths::LMP_DATE).and_then(parse_date);
	let last_menstrual_period_date_null_flavor =
		first_value_root(&mut xpath, DPatientPaths::LMP_DATE_NULL_FLAVOR);
	let race_code = first_value_root(&mut xpath, DPatientPaths::RACE_CODE);
	let race_code_null_flavor =
		first_value_root(&mut xpath, DPatientPaths::RACE_CODE_NULL_FLAVOR);
	let ethnicity_code = first_value_root(&mut xpath, DPatientPaths::ETHNICITY_CODE);
	let ethnicity_code_null_flavor =
		first_value_root(&mut xpath, DPatientPaths::ETHNICITY_CODE_NULL_FLAVOR);
	let medical_history_text =
		first_text_root(&mut xpath, DPatientPaths::MEDICAL_HISTORY_TEXT);
	let medical_history_text_null_flavor = first_value_root(
		&mut xpath,
		DPatientPaths::MEDICAL_HISTORY_TEXT_NULL_FLAVOR,
	);
	let concomitant_therapy = parse_bool_value(first_value_root(
		&mut xpath,
		DPatientPaths::CONCOMITANT_THERAPY_VALUE,
	));

	if patient_initials.is_none()
		&& sex.is_none()
		&& age_at_time_of_onset.is_none()
		&& gestation_period.is_none()
		&& weight_kg.is_none()
		&& height_cm.is_none()
		&& patient_initials_null_flavor.is_none()
		&& birth_date_null_flavor.is_none()
		&& age_at_time_of_onset_null_flavor.is_none()
		&& sex_null_flavor.is_none()
		&& last_menstrual_period_date_null_flavor.is_none()
		&& race_code_null_flavor.is_none()
		&& ethnicity_code_null_flavor.is_none()
	{
		return Ok(None);
	}

	Ok(Some(DPatientImport {
		patient_initials,
		patient_initials_null_flavor,
		birth_date,
		birth_date_null_flavor,
		sex,
		sex_null_flavor,
		age_at_time_of_onset,
		age_at_time_of_onset_null_flavor,
		age_unit,
		gestation_period,
		gestation_period_unit,
		age_group,
		weight_kg,
		height_cm,
		race_code,
		race_code_null_flavor,
		ethnicity_code,
		ethnicity_code_null_flavor,
		last_menstrual_period_date,
		last_menstrual_period_date_null_flavor,
		medical_history_text,
		medical_history_text_null_flavor,
		concomitant_therapy,
	}))
}

fn first_value_root(xpath: &mut Context, path: &str) -> Option<String> {
	match xpath.findvalue(path, None) {
		Ok(value) if !value.trim().is_empty() => Some(value),
		_ => None,
	}
}

fn first_text_root(xpath: &mut Context, path: &str) -> Option<String> {
	match xpath.findvalue(path, None) {
		Ok(value) if !value.trim().is_empty() => Some(value),
		_ => None,
	}
}

fn normalize_code(
	value: Option<String>,
	allowed: &[&str],
	_field: &str,
) -> Option<String> {
	let candidate = value?;
	if allowed.iter().any(|v| *v == candidate) {
		Some(candidate)
	} else {
		None
	}
}

fn normalize_code3(value: Option<String>, _field: &str) -> Option<String> {
	value.and_then(|v| if v.len() <= 3 { Some(v) } else { None })
}

fn normalize_sex_code(value: Option<String>) -> Option<String> {
	match value.as_deref() {
		Some("1") | Some("2") | Some("0") => value,
		_ => None,
	}
}

fn parse_bool_value(value: Option<String>) -> Option<bool> {
	value.and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
		"true" | "1" | "yes" => Some(true),
		"false" | "0" | "no" => Some(false),
		_ => None,
	})
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
