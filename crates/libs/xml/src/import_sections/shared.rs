use crate::error::Error;
use crate::import_constraint;
use crate::Result;
use libxml::parser::{Parser, ParserOptions};
use libxml::tree::{Document, Node};
use libxml::xpath::Context;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use std::collections::HashMap;
use time::Month;

#[derive(Debug, Default)]
pub(crate) struct ImportIdMap {
	by_xml_id: HashMap<String, Uuid>,
	by_normalized_xml_id: HashMap<String, Option<Uuid>>,
	by_sequence: Vec<Uuid>,
}

impl ImportIdMap {
	pub(crate) fn insert_xml_id(&mut self, xml_id: String, id: Uuid) {
		let normalized = normalize_reference_id(&xml_id);
		self.by_normalized_xml_id
			.entry(normalized)
			.and_modify(|existing| {
				if *existing != Some(id) {
					*existing = None;
				}
			})
			.or_insert(Some(id));
		self.by_xml_id.insert(xml_id, id);
	}

	pub(crate) fn push_sequence(&mut self, id: Uuid) {
		self.by_sequence.push(id);
	}

	pub(crate) fn resolve(
		&self,
		xml_id: Option<String>,
		sequence: Option<i32>,
	) -> Option<Uuid> {
		if let Some(xml_id) = xml_id {
			if let Some(id) = self.by_xml_id.get(&xml_id).copied() {
				return Some(id);
			}
			if let Some(id) = self
				.by_normalized_xml_id
				.get(&normalize_reference_id(&xml_id))
				.copied()
				.flatten()
			{
				return Some(id);
			}
		}
		if let Some(seq) = sequence {
			if seq > 0 {
				let idx = (seq - 1) as usize;
				if idx < self.by_sequence.len() {
					return Some(self.by_sequence[idx]);
				}
			}
		}
		None
	}
}

fn normalize_reference_id(value: &str) -> String {
	value
		.chars()
		.filter(|character| character.is_ascii_alphanumeric())
		.flat_map(char::to_lowercase)
		.collect()
}

#[cfg(test)]
mod import_id_map_tests {
	use super::ImportIdMap;
	use sqlx::types::Uuid;

	#[test]
	fn unresolved_reference_does_not_fall_back_to_first_row() {
		let mut map = ImportIdMap::default();
		map.push_sequence(Uuid::new_v4());

		assert_eq!(map.resolve(Some("unknown".to_string()), None), None);
		assert_eq!(map.resolve(None, None), None);
		assert_eq!(map.resolve(None, Some(2)), None);
	}

	#[test]
	fn resolves_unique_reference_with_different_separators() {
		let mut map = ImportIdMap::default();
		let id = Uuid::new_v4();
		map.insert_xml_id("rid-1".to_string(), id);

		assert_eq!(map.resolve(Some("r-id1".to_string()), None), Some(id));
	}

	#[test]
	fn ambiguous_normalized_reference_stays_unresolved() {
		let mut map = ImportIdMap::default();
		map.insert_xml_id("rid-1".to_string(), Uuid::new_v4());
		map.insert_xml_id("r-id1".to_string(), Uuid::new_v4());

		assert_eq!(map.resolve(Some("RID1".to_string()), None), None);
	}
}

pub(crate) fn first_attr(
	xpath: &mut Context,
	node: &Node,
	expr: &str,
	attr: &str,
) -> Option<String> {
	let expr = format!("{expr}/@{attr}");
	xpath
		.findvalues(&expr, Some(node))
		.ok()?
		.into_iter()
		.find(|v| !v.trim().is_empty())
}

pub(crate) fn first_value(
	xpath: &mut Context,
	node: &Node,
	expr: &str,
) -> Option<String> {
	xpath
		.findvalues(expr, Some(node))
		.ok()?
		.into_iter()
		.find(|v| !v.trim().is_empty())
}

pub(crate) fn first_text(
	xpath: &mut Context,
	node: &Node,
	expr: &str,
) -> Option<String> {
	let nodes = xpath.findnodes(expr, Some(node)).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

pub(crate) fn first_value_root(xpath: &mut Context, expr: &str) -> Option<String> {
	xpath
		.findvalues(expr, None)
		.ok()?
		.into_iter()
		.find(|v| !v.trim().is_empty())
}

pub(crate) fn first_text_root(xpath: &mut Context, expr: &str) -> Option<String> {
	let nodes = xpath.findnodes(expr, None).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

pub(crate) fn parse_bool_attr(
	xpath: &mut Context,
	node: &Node,
	expr: &str,
	attr: &str,
) -> Option<bool> {
	parse_bool_literal(first_attr(xpath, node, expr, attr).as_deref())
}

/// Case-insensitive true/false/1/0, without trimming.
pub(crate) fn parse_bool_literal(value: Option<&str>) -> Option<bool> {
	match value?.to_ascii_lowercase().as_str() {
		"true" | "1" => Some(true),
		"false" | "0" => Some(false),
		_ => None,
	}
}

/// Also accept yes/no, without trimming.
pub(crate) fn parse_bool_with_yes_no(value: Option<&str>) -> Option<bool> {
	let raw = value?;
	if raw.eq_ignore_ascii_case("yes") {
		Some(true)
	} else if raw.eq_ignore_ascii_case("no") {
		Some(false)
	} else {
		parse_bool_literal(value)
	}
}

pub(crate) fn parse_trimmed_bool_with_yes_no(value: Option<&str>) -> Option<bool> {
	parse_bool_with_yes_no(value.map(str::trim))
}

pub(crate) fn parse_xml_id_opt(value: Option<String>) -> Option<String> {
	let value = value?.trim().to_string();
	if value.is_empty() {
		return None;
	}
	Some(value)
}

pub(crate) fn normalize_code(
	value: Option<String>,
	allowed: &[&str],
	field: &str,
) -> Option<String> {
	match value {
		Some(v) => {
			let trimmed = v.trim();
			if allowed.contains(&trimmed) {
				return Some(trimmed.to_string());
			}
			let digit = trimmed.chars().next().filter(|c| c.is_ascii_digit());
			if let Some(d) = digit {
				let s = d.to_string();
				if allowed.contains(&s.as_str()) {
					eprintln!(
						"[import_e2b_xml] coercing {field} value={trimmed} -> {s}"
					);
					return Some(s);
				}
			}
			eprintln!("[import_e2b_xml] dropping invalid {field} value={trimmed}");
			None
		}
		None => None,
	}
}

pub(crate) fn normalize_iso2(value: Option<String>, field: &str) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	let upper = v.to_ascii_uppercase();
	if len == 2 && upper.chars().all(|c| c.is_ascii_uppercase()) {
		Some(upper)
	} else {
		tracing::warn!(field, value = %v, len, "dropping invalid ISO-3166-1 alpha-2");
		None
	}
}

pub(crate) fn telecom_first_in_node(
	xpath: &mut Context,
	node: &Node,
	prefix: &str,
) -> Option<String> {
	let values = xpath.findvalues(".//hl7:telecom/@value", Some(node)).ok()?;
	for value in values {
		let value = value.trim();
		if value.starts_with(prefix) {
			return Some(value.trim_start_matches(prefix).to_string());
		}
	}
	None
}

pub(crate) fn parse_date(value: String) -> Option<Date> {
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

#[cfg(test)]
#[test]
fn parse_date_preserves_existing_normalization_and_calendar_checks() {
	let leap_day = Date::from_calendar_date(2024, Month::February, 29).unwrap();
	for input in ["20240229", "2024-02-29", "20240229123456", "한2024/02/29"] {
		assert_eq!(parse_date(input.to_string()), Some(leap_day), "{input}");
	}
	for input in [
		"", "invalid", "2024", "202402", "20230229", "20241301", "20240100",
	] {
		assert_eq!(parse_date(input.to_string()), None, "{input}");
	}
}

pub(crate) fn normalize_message_date(value: String) -> Option<String> {
	let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
	if digits.len() < 14 {
		return None;
	}
	Some(digits[0..14].to_string())
}

pub(crate) fn make_import_message_number(base: &str, case_id: Uuid) -> String {
	let suffix = case_id.to_string();
	let max_base = 100usize.saturating_sub(1 + suffix.len());
	let truncated = if base.len() > max_base {
		base[..max_base].to_string()
	} else {
		base.to_string()
	};
	format!("{truncated}-{suffix}")
}

#[derive(Debug)]
pub(crate) struct MessageHeaderExtract {
	pub(crate) message_number: Option<String>,
	pub(crate) message_sender: Option<String>,
	pub(crate) message_receiver: Option<String>,
	pub(crate) message_date: Option<String>,
	pub(crate) batch_number: Option<String>,
	pub(crate) batch_sender: Option<String>,
	pub(crate) batch_receiver: Option<String>,
}

pub(crate) fn extract_message_header(xml: &[u8]) -> Result<MessageHeaderExtract> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let doc = parse_import_xml(xml_str)?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let mut first_value = |expr: &str| -> Result<Option<String>> {
		Ok(xpath
			.findvalues(expr, None)
			.map_err(|_| Error::InvalidXml {
				message: format!("Failed to query message header path: {expr}"),
				line: None,
				column: None,
			})?
			.into_iter()
			.find(|v| !v.trim().is_empty()))
	};

	Ok(MessageHeaderExtract {
		message_number: first_value("//hl7:PORR_IN049016UV/hl7:id/@extension")?,
		message_sender: first_value(
			"//hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id/@extension",
		)?,
		message_receiver: first_value(
			"//hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension",
		)?,
		message_date: first_value("//hl7:PORR_IN049016UV/hl7:creationTime/@value")?,
		batch_number: first_value("/hl7:MCCI_IN200100UV01/hl7:id/@extension")?,
		batch_sender: first_value(
			"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id/@extension",
		)?,
		batch_receiver: first_value(
			"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension",
		)?,
	})
}

pub(crate) fn extract_safety_report_id(xml: &[u8]) -> Result<String> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let doc = parse_import_xml(xml_str)?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let candidates = xpath
		.findvalues(
			"//hl7:investigationEvent[@classCode='INVSTG'][@moodCode='EVN']/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']/@extension",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query safety_report_id".to_string(),
			line: None,
			column: None,
		})?;
	for value in candidates {
		if !value.trim().is_empty() {
			import_constraint::string(
				"safetyReportId",
				Some(&value),
				None,
				input_contracts::generated::c::c_1_1,
			)?;
			return Ok(value);
		}
	}

	Err(Error::InvalidXml {
		message: "safety_report_id not found".to_string(),
		line: None,
		column: None,
	})
}

fn parse_import_xml(xml: &str) -> Result<Document> {
	Parser::default()
		.parse_string_with_options(
			xml,
			ParserOptions {
				recover: false,
				no_error: true,
				no_warning: true,
				..Default::default()
			},
		)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})
}

#[cfg(test)]
mod tests {
	use super::{
		extract_safety_report_id, parse_bool_literal, parse_bool_with_yes_no,
		parse_trimmed_bool_with_yes_no,
	};
	use crate::import_sections::d_patient::helpers::parse_patient_death;

	#[test]
	fn boolean_parsers_preserve_distinct_input_contracts() {
		for (raw, expected) in [
			(None, [None, None, None]),
			(Some(""), [None, None, None]),
			(Some("true"), [Some(true), Some(true), Some(true)]),
			(Some("TrUe"), [Some(true), Some(true), Some(true)]),
			(Some("1"), [Some(true), Some(true), Some(true)]),
			(Some("false"), [Some(false), Some(false), Some(false)]),
			(Some("FaLsE"), [Some(false), Some(false), Some(false)]),
			(Some("0"), [Some(false), Some(false), Some(false)]),
			(Some("YeS"), [None, Some(true), Some(true)]),
			(Some("nO"), [None, Some(false), Some(false)]),
			(Some(" true "), [None, None, Some(true)]),
			(Some("\t0\n"), [None, None, Some(false)]),
			(Some(" YES "), [None, None, Some(true)]),
			(Some("\u{a0}no\u{a0}"), [None, None, Some(false)]),
			(Some("2"), [None, None, None]),
			(Some("truefalse"), [None, None, None]),
			(Some("ＴＲＵＥ"), [None, None, None]),
		] {
			assert_eq!(
				[
					parse_bool_literal(raw),
					parse_bool_with_yes_no(raw),
					parse_trimmed_bool_with_yes_no(raw),
				],
				expected,
				"{raw:?}"
			);
		}
	}

	#[test]
	fn extract_safety_report_id_reads_matching_c_1_1_and_n_2_r_1() {
		let xml = br#"
			<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3">
				<PORR_IN049016UV>
					<id root="2.16.840.1.113883.3.989.2.1.3.1" extension="CASE-C-1-1"/>
					<controlActProcess>
						<subject>
							<investigationEvent classCode="INVSTG" moodCode="EVN">
								<id root="2.16.840.1.113883.3.989.2.1.3.1" extension="CASE-C-1-1"/>
							</investigationEvent>
						</subject>
					</controlActProcess>
				</PORR_IN049016UV>
			</MCCI_IN200100UV01>
		"#;

		let extracted = extract_safety_report_id(xml).expect("extract C.1.1");

		assert_eq!(extracted, "CASE-C-1-1");
	}

	#[test]
	fn rejects_malformed_xml_before_field_validation() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><id root="2.16.840.1.113883.3.989.2.1.3.1" extension="CASE"/><value xsi:type="CE" xsi:type="CE"/></PORR_IN049016UV></MCCI_IN200100UV01>"#;

		let error = extract_safety_report_id(xml).expect_err("malformed XML");
		assert!(error.to_string().contains("XML parse error"));
	}

	#[test]
	fn extract_safety_report_id_accepts_n_2_r_1_mismatch() {
		let xml = br#"
			<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3">
				<PORR_IN049016UV>
					<id extension="MESSAGE-ID"/>
					<controlActProcess><subject>
						<investigationEvent classCode="INVSTG" moodCode="EVN">
							<id root="2.16.840.1.113883.3.989.2.1.3.1" extension="CASE-ID"/>
						</investigationEvent>
					</subject></controlActProcess>
				</PORR_IN049016UV>
			</MCCI_IN200100UV01>
		"#;

		assert_eq!(extract_safety_report_id(xml).unwrap(), "CASE-ID");
	}

	#[test]
	fn accepts_fda_scenario_2_identifiers() {
		let xml = include_bytes!(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/../../../docs/exporter/fda/FAERS2022Scenario2.xml"
		));

		assert_eq!(
			extract_safety_report_id(xml).unwrap(),
			"US-APHARMA-8744554B-UPDATE-TESTING222"
		);
	}

	#[test]
	fn parse_patient_death_reads_reported_and_autopsy_comments() {
		let xml = br#"
			<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3">
				<PORR_IN049016UV>
					<controlActProcess>
						<subject>
							<investigationEvent>
								<subjectOf2>
									<observation>
										<code code="32"/>
										<value code="10042984" codeSystemVersion="27.1">
											<originalText>Progressive multifocal leukoencephalopathy</originalText>
										</value>
									</observation>
								</subjectOf2>
								<subjectOf2>
									<observation>
										<code code="5"/>
										<value value="true"/>
										<outboundRelationship2>
											<observation>
												<code code="8"/>
												<value code="10011906" codeSystemVersion="27.1">
													<originalText>What we learned during the autopsy</originalText>
												</value>
											</observation>
										</outboundRelationship2>
									</observation>
								</subjectOf2>
							</investigationEvent>
						</subject>
					</controlActProcess>
				</PORR_IN049016UV>
			</MCCI_IN200100UV01>
		"#;

		let death = parse_patient_death(xml)
			.expect("parse death")
			.expect("death block");
		assert_eq!(
			death.reported_causes[0].comments.as_deref(),
			Some("Progressive multifocal leukoencephalopathy")
		);
		assert_eq!(
			death.autopsy_causes[0].comments.as_deref(),
			Some("What we learned during the autopsy")
		);
	}
}
