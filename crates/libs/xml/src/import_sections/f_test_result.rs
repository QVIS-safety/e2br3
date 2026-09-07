// Section F importer (Tests and Procedures) - FDA mapping.

use crate::error::Error;
use crate::import_constraint;
use crate::import_sections::shared::parse_date;
use crate::mapping::fda::f_test_result::FTestResultPaths;
use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::test_result::{TestResultBmc, TestResultForCreate};
use lib_core::model::ModelManager;
use libxml::parser::Parser;
use libxml::tree::Node;
use libxml::xpath::Context;
use sqlx::types::time::Date;

#[derive(Debug)]
pub struct FTestResultImport {
	pub test_name: String,
	pub test_date: Option<Date>,
	pub test_date_null_flavor: Option<String>,
	pub test_meddra_version: Option<String>,
	pub test_meddra_code: Option<String>,
	pub test_result_code: Option<String>,
	pub test_result_value: Option<String>,
	pub test_result_qualifier: Option<String>,
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
	for node in nodes {
		let (test_date, test_date_null_flavor) = read_f_r_1(&mut xpath, &node)?;
		let test_name = read_f_r_2_1(&mut xpath, &node)?;
		let (test_result_value, test_result_qualifier) =
			read_f_r_3_2(&mut xpath, &node)?;

		items.push(FTestResultImport {
			test_name,
			test_date,
			test_date_null_flavor,
			test_meddra_version: read_f_r_2_2a(&mut xpath, &node)?,
			test_meddra_code: read_f_r_2_2b(&mut xpath, &node)?,
			test_result_code: read_f_r_3_1(&mut xpath, &node)?,
			test_result_value,
			test_result_qualifier,
			test_result_unit: read_f_r_3_3(&mut xpath, &node)?,
			result_unstructured: read_f_r_3_4(&mut xpath, &node)?,
			normal_low_value: read_f_r_4(&mut xpath, &node)?,
			normal_high_value: read_f_r_5(&mut xpath, &node)?,
			comments: read_f_r_6(&mut xpath, &node)?,
			more_info_available: read_f_r_7(&mut xpath, &node)?,
		});
	}

	Ok(items)
}

pub(crate) async fn import_section_f(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: sqlx::types::Uuid,
) -> Result<()> {
	let tests = parse_f_test_results(xml)?;
	set_full_context_dbx(mm.dbx(), ctx.user_id(), ctx.organization_id(), ctx.role())
		.await
		.map_err(Error::Model)?;
	for (idx, entry) in tests.into_iter().enumerate() {
		TestResultBmc::create(
			ctx,
			mm,
			TestResultForCreate {
				case_id,
				sequence_number: (idx + 1) as i32,
				test_date: entry.test_date,
				test_date_null_flavor: entry.test_date_null_flavor,
				test_name: entry.test_name,
				test_meddra_version: entry.test_meddra_version,
				test_meddra_code: entry.test_meddra_code,
				test_result_code: entry.test_result_code,
				test_result_value: entry.test_result_value,
				test_result_qualifier: entry.test_result_qualifier,
				test_result_unit: entry.test_result_unit,
				result_unstructured: entry.result_unstructured,
				normal_low_value: entry.normal_low_value,
				normal_high_value: entry.normal_high_value,
				comments: entry.comments,
				more_info_available: entry.more_info_available,
			},
		)
		.await?;
	}
	Ok(())
}

/// e2b:F.r.1
fn read_f_r_1(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<Date>, Option<String>)> {
	let value =
		first_attr(xpath, node, FTestResultPaths::TEST_DATE).and_then(parse_date);
	let null_flavor =
		first_attr(xpath, node, FTestResultPaths::TEST_DATE_NULL_FLAVOR);
	let raw = first_attr(xpath, node, FTestResultPaths::TEST_DATE);
	import_constraint::string(
		"testDate",
		raw.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::f::f_r_1,
	)?;
	import_constraint::string(
		"testDateNullFlavor",
		None,
		None,
		input_contracts::generated::f::f_r_1,
	)?;
	Ok((value, null_flavor))
}

/// e2b:F.r.2.1
fn read_f_r_2_1(xpath: &mut Context, node: &Node) -> Result<String> {
	let value = first_text(xpath, node, FTestResultPaths::TEST_NAME);
	import_constraint::string(
		"testName",
		value.as_deref(),
		None,
		input_contracts::generated::f::f_r_2_1,
	)?;
	Ok(value.unwrap_or_default())
}

/// e2b:F.r.2.2a
fn read_f_r_2_2a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, FTestResultPaths::TEST_MEDDRA_VERSION),
		"testMeddraVersion",
		input_contracts::generated::f::f_r_2_2a,
	)
}

/// e2b:F.r.2.2b
fn read_f_r_2_2b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, FTestResultPaths::TEST_MEDDRA_CODE),
		"testMeddraCode",
		input_contracts::generated::f::f_r_2_2b,
	)
}

/// e2b:F.r.3.1
fn read_f_r_3_1(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, FTestResultPaths::RESULT_CODE),
		"testResultCode",
		input_contracts::generated::f::f_r_3_1,
	)
}

/// e2b:F.r.3.2
fn read_f_r_3_2(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	let center = first_attr(xpath, node, FTestResultPaths::RESULT_VALUE);
	let low_value = first_attr(xpath, node, FTestResultPaths::RESULT_LOW_VALUE);
	let low_null = first_attr(xpath, node, FTestResultPaths::RESULT_LOW_NULL_FLAVOR);
	let high_value = first_attr(xpath, node, FTestResultPaths::RESULT_HIGH_VALUE);
	let high_null =
		first_attr(xpath, node, FTestResultPaths::RESULT_HIGH_NULL_FLAVOR);
	let (value, qualifier) = if let Some(value) = center {
		(Some(value), Some("EQ".to_string()))
	} else if low_null.as_deref() == Some("NINF") && high_value.is_some() {
		let inclusive =
			first_attr(xpath, node, FTestResultPaths::RESULT_HIGH_INCLUSIVE)
				.as_deref() == Some("true");
		(
			high_value,
			Some(if inclusive { "LE" } else { "LT" }.to_string()),
		)
	} else if high_null.as_deref() == Some("PINF") && low_value.is_some() {
		let inclusive =
			first_attr(xpath, node, FTestResultPaths::RESULT_LOW_INCLUSIVE)
				.as_deref() == Some("true");
		(
			low_value,
			Some(if inclusive { "GE" } else { "GT" }.to_string()),
		)
	} else {
		(None, None)
	};
	import_constraint::string(
		"testResult",
		value.as_deref(),
		None,
		input_contracts::generated::f::f_r_3_2,
	)?;
	Ok((value, qualifier))
}

/// e2b:F.r.3.3
fn read_f_r_3_3(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let unit_path = if first_attr(xpath, node, FTestResultPaths::RESULT_VALUE)
		.is_some()
	{
		FTestResultPaths::RESULT_UNIT
	} else if first_attr(xpath, node, FTestResultPaths::RESULT_HIGH_VALUE).is_some()
	{
		FTestResultPaths::RESULT_HIGH_UNIT
	} else if first_attr(xpath, node, FTestResultPaths::RESULT_LOW_VALUE).is_some() {
		FTestResultPaths::RESULT_LOW_UNIT
	} else {
		FTestResultPaths::RESULT_UNIT
	};
	input_string(
		first_attr(xpath, node, unit_path),
		"testUnit",
		input_contracts::generated::f::f_r_3_3,
	)
}

/// e2b:F.r.3.4
fn read_f_r_3_4(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, FTestResultPaths::RESULT_UNSTRUCTURED),
		"testResultUnstructured",
		input_contracts::generated::f::f_r_3_4,
	)
}

/// e2b:F.r.4
fn read_f_r_4(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, FTestResultPaths::NORMAL_LOW),
		"lowRange",
		input_contracts::generated::f::f_r_4,
	)
}

/// e2b:F.r.5
fn read_f_r_5(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, FTestResultPaths::NORMAL_HIGH),
		"highRange",
		input_contracts::generated::f::f_r_5,
	)
}

/// e2b:F.r.6
fn read_f_r_6(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, FTestResultPaths::COMMENTS),
		"comments",
		input_contracts::generated::f::f_r_6,
	)
}

/// e2b:F.r.7
fn read_f_r_7(xpath: &mut Context, node: &Node) -> Result<Option<bool>> {
	let value =
		parse_bool_value(first_attr(xpath, node, FTestResultPaths::MORE_INFO));
	import_constraint::boolean(
		"moreInformationAvailable",
		value,
		None,
		input_contracts::generated::f::f_r_7,
	)?;
	Ok(value)
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

fn parse_bool_value(value: Option<String>) -> Option<bool> {
	let val = value?;
	match val.to_ascii_lowercase().as_str() {
		"true" | "1" => Some(true),
		"false" | "0" => Some(false),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::parse_f_test_results;

	#[test]
	fn imports_missing_test_name_for_later_business_validation() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><organizer><code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><observation><effectiveTime value="20260810"/></observation></component></organizer></MCCI_IN200100UV01>"#;
		let results =
			parse_f_test_results(xml).expect("missing name is not a parse error");
		assert_eq!(results[0].test_name, "");
	}

	#[test]
	fn leaves_top_level_test_result_null_flavor_to_business_validation() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><organizer><code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><observation><code><originalText>Result</originalText></code><value nullFlavor="NINF"/></observation></component></organizer></MCCI_IN200100UV01>"#;
		parse_f_test_results(xml).expect("business validation runs later");
	}

	#[test]
	fn imports_upper_interval_bound_as_less_than() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><organizer><code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><observation><code><originalText>Result</originalText></code><value><low nullFlavor="NINF"/><high value="10" unit="mg/dL" inclusive="false"/></value></observation></component></organizer></MCCI_IN200100UV01>"#;
		let results = parse_f_test_results(xml).expect("parse interval");
		assert_eq!(results[0].test_result_value.as_deref(), Some("10"));
		assert_eq!(results[0].test_result_qualifier.as_deref(), Some("LT"));
		assert_eq!(results[0].test_result_unit.as_deref(), Some("mg/dL"));
	}

	#[test]
	fn imports_f_r_7_official_true_and_false_values() {
		for (raw, expected) in [("true", true), ("false", false)] {
			let xml = format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><organizer><code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><observation><code><originalText>Result</originalText></code><outboundRelationship2 typeCode="REFR"><observation><code code="25" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value value="{raw}"/></observation></outboundRelationship2></observation></component></organizer></MCCI_IN200100UV01>"#
			);
			let results = parse_f_test_results(xml.as_bytes()).expect("parse F.r.7");
			assert_eq!(results[0].more_info_available, Some(expected));
		}
	}

	#[test]
	fn ignores_hl7_high_interpretation_when_reading_e2b_result_code() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><organizer><code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><observation><code><originalText>ALT</originalText></code><value><center value="86" unit="U/L"/></value><interpretationCode code="H" codeSystem="2.16.840.1.113883.5.83"/><referenceRange><observationRange><value value="56"/><interpretationCode code="H" codeSystem="2.16.840.1.113883.5.83"/></observationRange></referenceRange></observation></component></organizer></MCCI_IN200100UV01>"#;

		let results = parse_f_test_results(xml).expect("parse HL7 interpretation");
		assert_eq!(results[0].test_result_code, None);
		assert_eq!(results[0].test_result_value.as_deref(), Some("86"));
	}
}
