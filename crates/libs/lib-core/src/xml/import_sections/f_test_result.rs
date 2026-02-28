// Section F importer (Tests and Procedures) - FDA mapping.

use crate::xml::error::Error;
use crate::xml::mapping::fda::f_test_result::FTestResultPaths;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Node;
use libxml::xpath::Context;
use sqlx::types::time::Date;
use time::Month;

#[derive(Debug)]
pub struct FTestResultImport {
	pub test_name: String,
	pub test_date: Option<Date>,
	pub test_meddra_version: Option<String>,
	pub test_meddra_code: Option<String>,
	pub test_result_code: Option<String>,
	pub test_result_value: Option<String>,
	pub test_result_unit: Option<String>,
	pub result_unstructured: Option<String>,
	pub normal_low_value: Option<String>,
	pub normal_high_value: Option<String>,
	pub comments: Option<String>,
	pub more_info_available: Option<bool>,
}

pub fn parse_f_test_results(xml: &[u8]) -> Result<Vec<FTestResultImport>> {
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

	let nodes =
		xpath
			.findnodes(FTestResultPaths::TEST_NODE, None)
			.map_err(|_| Error::InvalidXml {
				message: "Failed to query test results".to_string(),
				line: None,
				column: None,
			})?;

	let mut items = Vec::new();
	for (idx, node) in nodes.into_iter().enumerate() {
		let test_name = first_text(&mut xpath, &node, FTestResultPaths::TEST_NAME)
			.or_else(|| {
				first_attr(&mut xpath, &node, FTestResultPaths::TEST_NAME_DISPLAY)
			})
			.unwrap_or_else(|| {
				eprintln!(
					"[import_e2b_xml] test_results[{idx}] missing F.r.2 test_name; importing empty test_name for downstream validation"
				);
				String::new()
			});
		let test_meddra_code =
			first_attr(&mut xpath, &node, FTestResultPaths::TEST_MEDDRA_CODE);
		let test_meddra_version = clamp_str(
			first_attr(&mut xpath, &node, FTestResultPaths::TEST_MEDDRA_VERSION),
			10,
		);
		let test_date = first_attr(&mut xpath, &node, FTestResultPaths::TEST_DATE)
			.and_then(parse_date);
		let test_result_code =
			first_attr(&mut xpath, &node, FTestResultPaths::RESULT_CODE);
		let test_result_value = first_attr(
			&mut xpath,
			&node,
			FTestResultPaths::RESULT_VALUE,
		)
		.or_else(|| {
			first_attr(&mut xpath, &node, FTestResultPaths::RESULT_VALUE_FALLBACK)
		});
		let test_result_unit = first_attr(
			&mut xpath,
			&node,
			FTestResultPaths::RESULT_UNIT,
		)
		.or_else(|| {
			first_attr(&mut xpath, &node, FTestResultPaths::RESULT_UNIT_FALLBACK)
		});
		let result_unstructured =
			first_text(&mut xpath, &node, FTestResultPaths::RESULT_UNSTRUCTURED);
		let normal_low_value =
			first_attr(&mut xpath, &node, FTestResultPaths::NORMAL_LOW);
		let normal_high_value =
			first_attr(&mut xpath, &node, FTestResultPaths::NORMAL_HIGH);
		let comments = first_text(&mut xpath, &node, FTestResultPaths::COMMENTS);
		let more_info_available = parse_bool_value(first_attr(
			&mut xpath,
			&node,
			FTestResultPaths::MORE_INFO,
		));

		items.push(FTestResultImport {
			test_name,
			test_date,
			test_meddra_version,
			test_meddra_code,
			test_result_code,
			test_result_value,
			test_result_unit,
			result_unstructured,
			normal_low_value,
			normal_high_value,
			comments,
			more_info_available,
		});
	}

	Ok(items)
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
