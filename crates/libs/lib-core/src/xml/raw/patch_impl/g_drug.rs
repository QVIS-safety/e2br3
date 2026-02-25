use super::*;

pub fn patch_g_drugs(
	raw_xml: &[u8],
	drugs: &[DrugInformation],
	substances: &[DrugActiveSubstance],
	dosages: &[DosageInformation],
	indications: &[DrugIndication],
	characteristics: &[DrugDeviceCharacteristic],
	assessments: &[DrugReactionAssessment],
	relatedness: &[RelatednessAssessment],
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

	ensure_primary_role(&mut doc, &parser, &mut xpath)?;
	remove_nodes(
		&mut xpath,
		"//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]",
	);
	// Remove template causality blocks so we don't leak hardcoded product/reaction IDs.
	remove_nodes(
		&mut xpath,
		"//hl7:adverseEventAssessment/hl7:component[hl7:causalityAssessment]",
	);

	for drug in drugs {
		let subs: Vec<&DrugActiveSubstance> =
			substances.iter().filter(|s| s.drug_id == drug.id).collect();
		let doses: Vec<&DosageInformation> =
			dosages.iter().filter(|d| d.drug_id == drug.id).collect();
		let inds: Vec<&DrugIndication> = indications
			.iter()
			.filter(|i| i.drug_id == drug.id)
			.collect();
		let chars: Vec<&DrugDeviceCharacteristic> = characteristics
			.iter()
			.filter(|c| c.drug_id == drug.id)
			.collect();
		let drug_assessments: Vec<&DrugReactionAssessment> = assessments
			.iter()
			.filter(|a| a.drug_id == drug.id)
			.collect();
		let fragment = drug_fragment(drug, &subs, &doses, &inds, &chars);
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:primaryRole",
			&fragment,
		)?;
		let role_fragment = causality_role_fragment(drug)?;
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:adverseEventAssessment",
			&role_fragment,
		)?;
		for assessment in drug_assessments {
			let rows: Vec<&RelatednessAssessment> = relatedness
				.iter()
				.filter(|r| r.drug_reaction_assessment_id == assessment.id)
				.collect();
			if rows.is_empty() {
				continue;
			}
			for row in rows {
				let related_fragment =
					relatedness_fragment(drug.id, assessment, row);
				append_fragment_child(
					&mut doc,
					&parser,
					&mut xpath,
					"//hl7:adverseEventAssessment",
					&related_fragment,
				)?;
			}
		}
	}

	Ok(doc.to_string())
}

fn relatedness_fragment(
	drug_id: sqlx::types::Uuid,
	assessment: &DrugReactionAssessment,
	relatedness: &RelatednessAssessment,
) -> String {
	let mut out = String::new();
	out.push_str("<component typeCode=\"COMP\"><causalityAssessment classCode=\"OBS\" moodCode=\"EVN\">");
	out.push_str(
		"<code code=\"39\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"causality\"/>",
	);
	out.push_str("<subject1 typeCode=\"SBJ\"><adverseEffectReference classCode=\"OBS\" moodCode=\"EVN\"><id root=\"");
	out.push_str(&assessment.reaction_id.to_string());
	out.push_str("\"/></adverseEffectReference></subject1>");
	out.push_str("<subject2 typeCode=\"SBJ\"><productUseReference classCode=\"SBADM\" moodCode=\"EVN\"><id root=\"");
	out.push_str(&drug_id.to_string());
	out.push_str("\"/></productUseReference></subject2>");
	if let Some(source) = relatedness.source_of_assessment.as_deref() {
		let s = xml_escape(source);
		out.push_str("<author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code><originalText>");
		out.push_str(&s);
		out.push_str("</originalText></code></assignedEntity></author>");
	}
	if let Some(method) = relatedness.method_of_assessment.as_deref() {
		let m = xml_escape(method);
		out.push_str("<methodCode><originalText>");
		out.push_str(&m);
		out.push_str("</originalText></methodCode>");
	}
	if let Some(result) = relatedness.result_of_assessment.as_deref() {
		let r = xml_escape(result);
		out.push_str("<value xsi:type=\"CE\"><originalText>");
		out.push_str(&r);
		out.push_str("</originalText></value>");
	}
	out.push_str("</causalityAssessment></component>");
	out
}

fn xml_escape(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}

fn causality_role_fragment(drug: &DrugInformation) -> Result<String> {
	let role_code = normalize_drug_characterization(&drug.drug_characterization)
		.ok_or_else(|| Error::InvalidXml {
			message: format!(
				"ICH.G.k.1.REQUIRED: drug characterization missing or invalid for drug sequence {}",
				drug.sequence_number
			),
			line: None,
			column: None,
		})?;
	let display = drug_characterization_display_name(role_code);
	Ok(format!(
		"<component1 typeCode=\"COMP\">\
			<observationEvent classCode=\"OBS\" moodCode=\"EVN\">\
				<code code=\"20\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"interventionCharacterization\"/>\
				<value xsi:type=\"CE\" code=\"{role_code}\" displayName=\"{display}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.13\"/>\
			</observationEvent>\
		</component1>"
	))
}
