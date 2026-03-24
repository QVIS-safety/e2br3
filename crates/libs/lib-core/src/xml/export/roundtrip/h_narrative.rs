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

	ensure_investigation_text(&mut doc, &parser, &mut xpath)?;
	set_text_first(
		&mut xpath,
		"//hl7:investigationEvent/hl7:text",
		&narrative.case_narrative,
	);

	remove_nodes(
		&mut xpath,
		"//hl7:adverseEventAssessment/hl7:component1[hl7:observationEvent/hl7:code[@code='10']]",
	);

	if let Some(comments) = narrative.reporter_comments.as_deref() {
		let fragment = comment_fragment(comments, "3");
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:adverseEventAssessment",
			&fragment,
		)?;
	}
	if let Some(comments) = narrative.sender_comments.as_deref() {
		let fragment = comment_fragment(comments, "1");
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:adverseEventAssessment",
			&fragment,
		)?;
	}

	Ok(doc.to_string())
}
