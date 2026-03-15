// Section H importer (Narrative) - FDA mapping.

use crate::xml::error::Error;
use crate::xml::mapping::fda::h_narrative::HNarrativePaths;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::xpath::Context;

#[derive(Debug)]
pub struct HNarrativeImport {
	pub case_narrative: String,
	pub reporter_comments: Option<String>,
	pub sender_comments: Option<String>,
}

#[derive(Debug)]
pub struct HSenderDiagnosisImport {
	pub sequence_number: i32,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
}

#[derive(Debug)]
pub struct HCaseSummaryImport {
	pub sequence_number: i32,
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
}

pub fn parse_h_narrative(xml: &[u8]) -> Result<Option<HNarrativeImport>> {
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

	let case_narrative =
		first_text_root(&mut xpath, HNarrativePaths::CASE_NARRATIVE)
			.or_else(|| first_text_root(&mut xpath, "//hl7:component1//hl7:text"))
			.or_else(|| first_text_root(&mut xpath, "//hl7:text"))
			.ok_or_else(|| Error::InvalidXml {
				message: "ICH.H.1.REQUIRED: case narrative missing".to_string(),
				line: None,
				column: None,
			})?;
	let reporter_comments =
		first_text_root(&mut xpath, HNarrativePaths::REPORTER_COMMENTS);
	let sender_comments =
		first_text_root(&mut xpath, HNarrativePaths::SENDER_COMMENTS);

	Ok(Some(HNarrativeImport {
		case_narrative,
		reporter_comments,
		sender_comments,
	}))
}

pub fn parse_h_sender_diagnoses(xml: &[u8]) -> Result<Vec<HSenderDiagnosisImport>> {
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

	let nodes = xpath
		.findnodes(
			"//hl7:component1//hl7:observationEvent[hl7:code[@code='15'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query sender diagnoses".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for (idx, node) in nodes.into_iter().enumerate() {
		items.push(HSenderDiagnosisImport {
			sequence_number: (idx + 1) as i32,
			diagnosis_meddra_version: first_attr(
				&mut xpath,
				&node,
				"hl7:value",
				"codeSystemVersion",
			),
			diagnosis_meddra_code: first_attr(
				&mut xpath,
				&node,
				"hl7:value",
				"code",
			),
		});
	}

	Ok(items)
}

pub fn parse_h_case_summaries(xml: &[u8]) -> Result<Vec<HCaseSummaryImport>> {
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

	let nodes = xpath
		.findnodes(
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query case summaries".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for (idx, node) in nodes.into_iter().enumerate() {
		items.push(HCaseSummaryImport {
			sequence_number: (idx + 1) as i32,
			summary_type: None,
			language_code: normalize_lang2(first_attr(
				&mut xpath,
				&node,
				"hl7:value",
				"language",
			)),
			summary_text: first_text(&mut xpath, &node, "hl7:value"),
		});
	}

	Ok(items)
}

fn first_text_root(xpath: &mut Context, expr: &str) -> Option<String> {
	let nodes = xpath.findnodes(expr, None).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

fn normalize_lang2(value: Option<String>) -> Option<String> {
	let v = value?.trim().to_ascii_lowercase();
	if v.len() == 2 && v.chars().all(|c| c.is_ascii_lowercase()) {
		return Some(v);
	}

	match v.as_str() {
		"eng" => Some("en".to_string()),
		"jpn" => Some("ja".to_string()),
		"kor" => Some("ko".to_string()),
		"deu" | "ger" => Some("de".to_string()),
		"fra" | "fre" => Some("fr".to_string()),
		"spa" => Some("es".to_string()),
		"ita" => Some("it".to_string()),
		"por" => Some("pt".to_string()),
		"zho" | "chi" => Some("zh".to_string()),
		_ => None,
	}
}

fn first_text(
	xpath: &mut Context,
	node: &libxml::tree::Node,
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

fn first_attr(
	xpath: &mut Context,
	node: &libxml::tree::Node,
	expr: &str,
	attr: &str,
) -> Option<String> {
	let nodes = xpath.findnodes(expr, Some(node)).ok()?;
	for n in nodes {
		if let Some(value) = n.get_attribute(attr) {
			if !value.trim().is_empty() {
				return Some(value);
			}
		}
	}
	None
}
