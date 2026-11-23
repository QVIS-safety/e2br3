// Section C importer (Safety Report Identification) - FDA mapping.

use crate::error::Error;
use crate::import_constraint::Context;
use crate::import_sections::shared::parse_trimmed_bool_with_yes_no;
use crate::mapping::fda::c_safety_report::CSafetyReportPaths;
use crate::Result;

mod helpers;
mod runtime;
pub use runtime::apply_c_safety_report_import_settings;
pub(crate) use runtime::import_section_c;

#[derive(Debug)]
pub struct CSafetyReportImport {
	pub transmission_date: Option<String>,
	pub report_type: Option<String>,
	pub date_first_received_from_source: Option<String>,
	pub date_of_most_recent_information: Option<String>,
	pub fulfil_expedited_criteria: Option<bool>,
	pub fulfil_expedited_criteria_null_flavor: Option<String>,
	pub additional_documents_available: Option<bool>,
	pub local_criteria_report_type: Option<String>,
	pub combination_product_report_indicator: Option<String>,
	pub combination_product_report_indicator_null_flavor: Option<String>,
	pub worldwide_unique_id: Option<String>,
	pub first_sender_type: Option<String>,
	pub other_case_identifiers_exist: Option<bool>,
	pub other_case_identifiers_exist_null_flavor: Option<String>,
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
	let doc = crate::import_sections::shared::parse_import_xml(xml_str)?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let result = (|| -> Result<_> {
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

		let transmission_date = read_c_1_2(&mut xpath)?;

		let report_type = read_c_1_3(&mut xpath)?;

		let date_first_received_from_source = read_c_1_4(&mut xpath)?;
		let date_of_most_recent_information = read_c_1_5(&mut xpath)?;
		let additional_documents_available = read_c_1_6_1(&mut xpath)?;
		let fulfil_expedited_criteria = read_c_1_7(&mut xpath)?;
		let local_criteria_report_type = read_fda_c_1_7_1(&mut xpath)?;
		let (
			combination_product_report_indicator,
			combination_product_report_indicator_null_flavor,
		) = read_fda_c_1_12(&mut xpath)?;
		let worldwide_unique_id = read_c_1_8_1(&mut xpath)?;
		let first_sender_type = read_c_1_8_2(&mut xpath)?;
		let (other_case_identifiers_exist, other_case_identifiers_exist_null_flavor) =
			read_c_1_9_1(&mut xpath)?;
		let nullification_code = read_c_1_11_1(&mut xpath)?;
		let nullification_reason = read_c_1_11_2(&mut xpath)?;

		Ok(Some(CSafetyReportImport {
			transmission_date,
			report_type,
			date_first_received_from_source,
			date_of_most_recent_information,
			fulfil_expedited_criteria,
			fulfil_expedited_criteria_null_flavor: first_value_root(
				&mut xpath,
				CSafetyReportPaths::FULFIL_EXPEDITED_NULL_FLAVOR,
			),
			additional_documents_available,
			local_criteria_report_type,
			combination_product_report_indicator,
			combination_product_report_indicator_null_flavor,
			worldwide_unique_id,
			first_sender_type,
			other_case_identifiers_exist,
			other_case_identifiers_exist_null_flavor,
			nullification_code,
			nullification_reason,
		}))
	})()?;
	xpath.validate()?;
	Ok(result)
}

/// e2b:C.1.2
fn read_c_1_2(xpath: &mut Context) -> Result<Option<String>> {
	let raw = first_value_root(xpath, CSafetyReportPaths::DATE_OF_CREATION);
	let value = raw;
	xpath.inputs.string(
		"transmissionDate",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_2,
	);
	Ok(value)
}

/// e2b:C.1.3
fn read_c_1_3(xpath: &mut Context) -> Result<Option<String>> {
	let value = first_value_root(xpath, CSafetyReportPaths::TYPE_OF_REPORT_CODE);
	xpath.inputs.string(
		"reportType",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_3,
	);
	Ok(value)
}

/// e2b:C.1.4
fn read_c_1_4(xpath: &mut Context) -> Result<Option<String>> {
	let value = first_value_root(xpath, CSafetyReportPaths::DATE_FIRST_RECEIVED);
	xpath.inputs.string(
		"dateFirstReceivedFromSource",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_4,
	);
	Ok(value)
}

/// e2b:C.1.5
fn read_c_1_5(xpath: &mut Context) -> Result<Option<String>> {
	let value = first_value_root(xpath, CSafetyReportPaths::DATE_MOST_RECENT);
	xpath.inputs.string(
		"dateOfMostRecentInformation",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_5,
	);
	Ok(value)
}

/// e2b:C.1.6.1
fn read_c_1_6_1(xpath: &mut Context) -> Result<Option<bool>> {
	let raw =
		first_value_root(xpath, CSafetyReportPaths::ADDITIONAL_DOCUMENTS_AVAILABLE);
	let value = parse_trimmed_bool_with_yes_no(raw.as_deref());
	xpath.inputs.boolean(
		"additionalDocumentsAvailable",
		raw.as_deref(),
		value,
		None,
		input_contracts::generated::c::c_1_6_1,
	);
	Ok(value)
}

/// e2b:C.1.7
fn read_c_1_7(xpath: &mut Context) -> Result<Option<bool>> {
	let raw = first_value_root(xpath, CSafetyReportPaths::FULFIL_EXPEDITED);
	let value = parse_trimmed_bool_with_yes_no(raw.as_deref());
	let null_flavor =
		first_value_root(xpath, CSafetyReportPaths::FULFIL_EXPEDITED_NULL_FLAVOR);
	xpath.inputs.boolean(
		"fulfilExpeditedCriteria",
		raw.as_deref(),
		value,
		null_flavor.as_deref(),
		input_contracts::generated::c::c_1_7,
	);
	Ok(value)
}

/// e2b:FDA.C.1.7.1
fn read_fda_c_1_7_1(xpath: &mut Context) -> Result<Option<String>> {
	let value = first_value_root(
		xpath,
		CSafetyReportPaths::FDA_LOCAL_CRITERIA_REPORT_TYPE_CODE,
	);
	xpath.inputs.string(
		"localCriteriaReportType",
		value.as_deref(),
		None,
		input_contracts::generated::c::fda_c_1_7_1,
	);
	Ok(value)
}

/// e2b:FDA.C.1.12
fn read_fda_c_1_12(xpath: &mut Context) -> Result<(Option<String>, Option<String>)> {
	let raw = first_value_root(
		xpath,
		CSafetyReportPaths::FDA_COMBINATION_PRODUCT_INDICATOR_VALUE,
	);
	let null_flavor = first_value_root(
		xpath,
		CSafetyReportPaths::FDA_COMBINATION_PRODUCT_INDICATOR_NULL_FLAVOR,
	);
	xpath.inputs.string(
		"combinationProductReportIndicator",
		raw.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::c::fda_c_1_12,
	);
	Ok((raw, null_flavor))
}

/// e2b:C.1.8.1
fn read_c_1_8_1(xpath: &mut Context) -> Result<Option<String>> {
	let value = first_value_root(xpath, CSafetyReportPaths::WORLDWIDE_UNIQUE_ID_EXT);
	xpath.inputs.string(
		"worldwideUniqueId",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_8_1,
	);
	Ok(value)
}

/// e2b:C.1.8.2
fn read_c_1_8_2(xpath: &mut Context) -> Result<Option<String>> {
	let value = first_value_root(xpath, CSafetyReportPaths::FIRST_SENDER_TYPE);
	xpath.inputs.string(
		"firstSenderType",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_8_2,
	);
	Ok(value)
}

/// e2b:C.1.9.1
fn read_c_1_9_1(xpath: &mut Context) -> Result<(Option<bool>, Option<String>)> {
	let raw =
		first_value_root(xpath, CSafetyReportPaths::OTHER_CASE_IDENTIFIERS_EXIST);
	let value = parse_trimmed_bool_with_yes_no(raw.as_deref());
	let null_flavor = first_value_root(
		xpath,
		CSafetyReportPaths::OTHER_CASE_IDENTIFIERS_EXIST_NULL_FLAVOR,
	);
	xpath.inputs.boolean(
		"otherCaseIdentifiersExist",
		raw.as_deref(),
		value,
		null_flavor.as_deref(),
		input_contracts::generated::c::c_1_9_1,
	);
	Ok((value, null_flavor))
}

/// e2b:C.1.11.1
fn read_c_1_11_1(xpath: &mut Context) -> Result<Option<String>> {
	let value = first_value_root(xpath, CSafetyReportPaths::NULLIFICATION_CODE);
	xpath.inputs.string(
		"nullificationAmendmentCode",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_11_1,
	);
	Ok(value)
}

/// e2b:C.1.11.2
fn read_c_1_11_2(xpath: &mut Context) -> Result<Option<String>> {
	let value = first_text_root(xpath, CSafetyReportPaths::NULLIFICATION_REASON);
	xpath.inputs.string(
		"nullificationReason",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_11_2,
	);
	Ok(value)
}

fn first_value_root(xpath: &mut Context, path: &str) -> Option<String> {
	xpath.findvalues(path, None).ok()?.into_iter().next()
}

fn first_text_root(xpath: &mut Context, path: &str) -> Option<String> {
	crate::import_sections::shared::first_text_root(xpath, path)
}

#[cfg(test)]
mod tests {
	use super::parse_c_safety_report;

	#[test]
	fn import_accepts_n_2_r_4_c_1_2_mismatch() {
		let xml = br#"
			<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3">
				<PORR_IN049016UV>
					<creationTime value="20260810000000"/>
					<controlActProcess><effectiveTime value="20260809000000"/></controlActProcess>
				</PORR_IN049016UV>
			</MCCI_IN200100UV01>
		"#;

		let parsed = parse_c_safety_report(xml).unwrap().unwrap();
		assert_eq!(parsed.transmission_date.as_deref(), Some("20260809000000"));
	}

	#[test]
	fn import_accepts_c_1_7_null_flavor_ni() {
		let xml = br#"
			<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3">
				<PORR_IN049016UV>
					<controlActProcess>
						<subject><investigationEvent><component><observationEvent>
							<code code="23" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/>
							<value nullFlavor="NI"/>
						</observationEvent></component></investigationEvent></subject>
					</controlActProcess>
				</PORR_IN049016UV>
			</MCCI_IN200100UV01>
		"#;

		let parsed = parse_c_safety_report(xml).unwrap().unwrap();
		assert_eq!(parsed.fulfil_expedited_criteria, None);
		assert_eq!(
			parsed.fulfil_expedited_criteria_null_flavor.as_deref(),
			Some("NI")
		);
	}
}
