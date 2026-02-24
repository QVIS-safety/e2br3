// Section C importer (Safety Report Identification) - FDA mapping.

use crate::xml::error::Error;
use crate::xml::mapping::fda::c_safety_report::CSafetyReportPaths;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::time::Date;
use time::Month;

#[derive(Debug)]
pub struct CSafetyReportImport {
	pub transmission_date: Date,
	pub report_type: String,
	pub date_first_received_from_source: Date,
	pub date_of_most_recent_information: Date,
	pub fulfil_expedited_criteria: bool,
	pub local_criteria_report_type: Option<String>,
	pub combination_product_report_indicator: Option<String>,
	pub worldwide_unique_id: Option<String>,
	pub nullification_code: Option<String>,
	pub nullification_reason: Option<String>,
}

/// Parse Section C values using FDA/ICH mapping paths.
pub fn parse_c_safety_report(xml: &[u8]) -> Result<Option<CSafetyReportImport>> {
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

	let transmission_raw =
		first_value_root(&mut xpath, CSafetyReportPaths::DATE_OF_CREATION);
	let transmission_date = transmission_raw
		.and_then(parse_date)
		.unwrap_or_else(|| time::OffsetDateTime::now_utc().date());

	let report_type = normalize_code(
		first_value_root(&mut xpath, CSafetyReportPaths::TYPE_OF_REPORT_CODE),
		&["1", "2", "3", "4"],
	)
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.C.1.3.REQUIRED: type of report missing".to_string(),
		line: None,
		column: None,
	})?;

	let date_first_received_from_source =
		first_value_root(&mut xpath, CSafetyReportPaths::DATE_FIRST_RECEIVED)
			.and_then(parse_date)
			.unwrap_or(transmission_date);

	let date_of_most_recent_information =
		first_value_root(&mut xpath, CSafetyReportPaths::DATE_MOST_RECENT)
			.and_then(parse_date)
			.unwrap_or(transmission_date);

	let fulfil_expedited_criteria = parse_bool_value(first_value_root(
		&mut xpath,
		CSafetyReportPaths::FULFIL_EXPEDITED,
	))
	.unwrap_or(false);

	let local_criteria_report_type = normalize_code(
		first_value_root(
			&mut xpath,
			CSafetyReportPaths::FDA_LOCAL_CRITERIA_REPORT_TYPE_CODE,
		),
		&["1", "2", "3", "4", "5"],
	);

	let combination_product_report_indicator = clamp_str(
		first_value_root(
			&mut xpath,
			CSafetyReportPaths::FDA_COMBINATION_PRODUCT_INDICATOR_VALUE,
		),
		10,
	);

	let worldwide_unique_id = clamp_str(
		first_value_root(&mut xpath, CSafetyReportPaths::WORLDWIDE_UNIQUE_ID_EXT),
		100,
	);

	let nullification_code = normalize_code(
		first_value_root(&mut xpath, CSafetyReportPaths::NULLIFICATION_CODE),
		&["1", "2", "3", "4"],
	);

	let nullification_reason = clamp_str(
		first_text_root(&mut xpath, CSafetyReportPaths::NULLIFICATION_REASON),
		200,
	);

	Ok(Some(CSafetyReportImport {
		transmission_date,
		report_type,
		date_first_received_from_source,
		date_of_most_recent_information,
		fulfil_expedited_criteria,
		local_criteria_report_type,
		combination_product_report_indicator,
		worldwide_unique_id,
		nullification_code,
		nullification_reason,
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

fn normalize_code(value: Option<String>, allowed: &[&str]) -> Option<String> {
	let candidate = value?;
	if allowed.iter().any(|v| *v == candidate) {
		Some(candidate)
	} else {
		None
	}
}

fn clamp_str(value: Option<String>, max_len: usize) -> Option<String> {
	let mut value = value?;
	if value.len() > max_len {
		value.truncate(max_len);
	}
	Some(value)
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
