use super::*;

pub fn patch_h_narrative(
	raw_xml: &[u8],
	narrative: &NarrativeInformation,
) -> Result<String> {
	let xml_str = std::str::from_utf8(raw_xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let mut doc = parser
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

	if let Some(text) = narrative.case_narrative.as_deref() {
		write_h_1(&mut doc, &parser, &mut xpath, text)?;
	} else {
		remove_nodes(&mut xpath, "//hl7:investigationEvent/hl7:text");
	}

	remove_nodes(
		&mut xpath,
		"//hl7:adverseEventAssessment/hl7:component1[hl7:observationEvent/hl7:code[@code='10']]",
	);

	if let Some(comments) = narrative.reporter_comments.as_deref() {
		write_h_2(&mut doc, &parser, &mut xpath, comments)?;
	}
	if let Some(comments) = narrative.sender_comments.as_deref() {
		write_h_4(&mut doc, &parser, &mut xpath, comments)?;
	}

	Ok(doc.to_string())
}

/// e2b:H.1
fn write_h_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	case_narrative: &str,
) -> Result<()> {
	ensure_investigation_text(doc, parser, xpath)?;
	set_text_first(xpath, "//hl7:investigationEvent/hl7:text", case_narrative);
	Ok(())
}

/// e2b:H.2
fn write_h_2(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	reporter_comments: &str,
) -> Result<()> {
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:adverseEventAssessment",
		&write_h_2_or_h_4(reporter_comments, "3"),
	)
}

/// e2b:H.4
fn write_h_4(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	sender_comments: &str,
) -> Result<()> {
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:adverseEventAssessment",
		&write_h_2_or_h_4(sender_comments, "1"),
	)
}
