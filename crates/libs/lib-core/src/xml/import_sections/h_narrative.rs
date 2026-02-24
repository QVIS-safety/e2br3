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
