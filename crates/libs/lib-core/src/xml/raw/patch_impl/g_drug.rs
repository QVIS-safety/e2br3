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
		"//*[local-name()='adverseEventAssessment']/*[(local-name()='component' or local-name()='component1') and .//*[local-name()='causalityAssessment']]",
	);
	// Preserve schema element ordering by rebuilding component1 blocks after G patching.
	// Narrative patch (H) runs later and rehydrates comment component1 entries.
	remove_nodes(
		&mut xpath,
		"//*[local-name()='adverseEventAssessment']/*[local-name()='component1']",
	);

	let mut ordered_drugs: Vec<&DrugInformation> = drugs.iter().collect();
	ordered_drugs.sort_by_key(|drug| drug.sequence_number);

	for drug in ordered_drugs {
		let subs: Vec<&DrugActiveSubstance> = {
			let mut rows: Vec<&DrugActiveSubstance> =
				substances.iter().filter(|s| s.drug_id == drug.id).collect();
			rows.sort_by_key(|row| row.sequence_number);
			rows
		};
		let doses: Vec<&DosageInformation> = {
			let mut rows: Vec<&DosageInformation> =
				dosages.iter().filter(|d| d.drug_id == drug.id).collect();
			rows.sort_by_key(|row| row.sequence_number);
			rows
		};
		let inds: Vec<&DrugIndication> = {
			let mut rows: Vec<&DrugIndication> = indications
				.iter()
				.filter(|i| i.drug_id == drug.id)
				.collect();
			rows.sort_by_key(|row| row.sequence_number);
			rows
		};
		let chars: Vec<&DrugDeviceCharacteristic> = {
			let mut rows: Vec<&DrugDeviceCharacteristic> = characteristics
				.iter()
				.filter(|c| c.drug_id == drug.id)
				.collect();
			rows.sort_by_key(|row| row.sequence_number);
			rows
		};
		let drug_assessments: Vec<&DrugReactionAssessment> = {
			let mut rows: Vec<&DrugReactionAssessment> = assessments
				.iter()
				.filter(|a| a.drug_id == drug.id)
				.collect();
			rows.sort_by_key(|row| row.reaction_id);
			rows
		};
		let fragment =
			drug_fragment(drug, &subs, &doses, &inds, &chars, &drug_assessments)?;
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:primaryRole",
			&fragment,
		)?;
		let causality_fragment = causality_role_fragment(drug)?;
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:adverseEventAssessment",
			&causality_fragment,
		)?;
		for assessment in drug_assessments {
			let mut rows: Vec<&RelatednessAssessment> = relatedness
				.iter()
				.filter(|r| r.drug_reaction_assessment_id == assessment.id)
				.collect();
			rows.sort_by_key(|row| row.sequence_number);
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
