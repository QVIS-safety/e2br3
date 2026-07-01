use crate::xml::error::Error;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Node;
use libxml::xpath::Context;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use std::collections::HashMap;
use time::Month;

#[derive(Debug, Default)]
pub(crate) struct ImportIdMap {
	by_xml_id: HashMap<Uuid, Uuid>,
	by_sequence: Vec<Uuid>,
}

impl ImportIdMap {
	pub(crate) fn first(&self) -> Option<Uuid> {
		self.by_sequence.first().copied()
	}

	pub(crate) fn insert_xml_id(&mut self, xml_id: Uuid, id: Uuid) {
		self.by_xml_id.insert(xml_id, id);
	}

	pub(crate) fn push_sequence(&mut self, id: Uuid) {
		self.by_sequence.push(id);
	}

	pub(crate) fn resolve(
		&self,
		xml_id: Option<Uuid>,
		sequence: Option<i32>,
	) -> Option<Uuid> {
		if let Some(id) = xml_id.and_then(|id| self.by_xml_id.get(&id).copied()) {
			return Some(id);
		}
		if let Some(seq) = sequence {
			if seq > 0 {
				let idx = (seq - 1) as usize;
				if idx < self.by_sequence.len() {
					return Some(self.by_sequence[idx]);
				}
			}
		}
		self.first()
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
	let val = first_attr(xpath, node, expr, attr)?;
	match val.to_ascii_lowercase().as_str() {
		"true" | "1" => Some(true),
		"false" | "0" => Some(false),
		_ => None,
	}
}

pub(crate) fn parse_bool_value(value: Option<String>) -> Option<bool> {
	let val = value?;
	match val.to_ascii_lowercase().as_str() {
		"true" | "1" | "yes" => Some(true),
		"false" | "0" | "no" => Some(false),
		_ => None,
	}
}

pub(crate) fn clamp_str(
	value: Option<String>,
	max: usize,
	field: &str,
) -> Option<String> {
	match value {
		Some(v) if v.len() > max => {
			eprintln!(
				"[import_e2b_xml] truncating {field} len={} -> {max}",
				v.len()
			);
			Some(v.chars().take(max).collect())
		}
		other => other,
	}
}

pub(crate) fn parse_uuid_opt(value: Option<String>) -> Option<Uuid> {
	let value = value?.trim().to_string();
	if value.is_empty() {
		return None;
	}
	Uuid::parse_str(&value).ok()
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

pub(crate) fn normalize_code3(value: Option<String>, field: &str) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	if (1..=3).contains(&len) && v.chars().all(|c| c.is_ascii_alphanumeric()) {
		Some(v)
	} else {
		tracing::warn!(field, value = %v, len, "dropping invalid 3-char code");
		None
	}
}

pub(crate) fn normalize_sex_code(value: Option<String>) -> Option<String> {
	let v = value?.trim().to_ascii_uppercase();
	match v.as_str() {
		"1" | "M" | "MALE" => Some("1".to_string()),
		"2" | "F" | "FEMALE" => Some("2".to_string()),
		"0" | "U" | "UNK" | "UNKNOWN" => Some("0".to_string()),
		_ => None,
	}
}

pub(crate) fn telecom_first(xpath: &mut Context, prefix: &str) -> Option<String> {
	let values = xpath.findvalues("//hl7:telecom/@value", None).ok()?;
	for value in values {
		let value = value.trim();
		if value.starts_with(prefix) {
			return Some(value.trim_start_matches(prefix).to_string());
		}
	}
	None
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
	pub(crate) batch_transmission: Option<String>,
}

pub(crate) fn extract_message_header(xml: &[u8]) -> Result<MessageHeaderExtract> {
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

	let mut first_value = |expr: &str| -> Option<String> {
		xpath
			.findvalues(expr, None)
			.ok()?
			.into_iter()
			.find(|v| !v.trim().is_empty())
	};

	Ok(MessageHeaderExtract {
		message_number: first_value("//hl7:PORR_IN049016UV/hl7:id/@extension"),
		message_sender: first_value(
			"//hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id/@extension",
		),
		message_receiver: first_value(
			"//hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension",
		),
		message_date: first_value("//hl7:PORR_IN049016UV/hl7:creationTime/@value"),
		batch_number: first_value("/hl7:MCCI_IN200100UV01/hl7:id/@extension"),
		batch_sender: first_value(
			"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id/@extension",
		),
		batch_receiver: first_value(
			"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension",
		),
		batch_transmission: first_value(
			"/hl7:MCCI_IN200100UV01/hl7:creationTime/@value",
		),
	})
}

pub(crate) fn extract_safety_report_id(xml: &[u8]) -> Result<String> {
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
			return Ok(value);
		}
	}

	Err(Error::InvalidXml {
		message: "safety_report_id not found".to_string(),
		line: None,
		column: None,
	})
}

#[cfg(test)]
mod tests {
	use super::extract_safety_report_id;
	use crate::xml::import_runtime::helpers::d::parse_patient_death;

	#[test]
	fn extract_safety_report_id_prefers_investigation_event_c_1_1() {
		let xml = br#"
			<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3">
				<PORR_IN049016UV>
					<id root="2.16.840.1.113883.3.989.2.1.3.1" extension="MESSAGE-ID"/>
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
	fn parse_patient_death_reads_reported_and_autopsy_comments() {
		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.expect("workspace root")
			.to_path_buf();
		let xml =
			std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario6.xml"))
				.expect("read sample xml");

		let death = parse_patient_death(&xml)
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
