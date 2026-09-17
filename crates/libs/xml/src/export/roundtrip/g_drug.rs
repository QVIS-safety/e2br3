use super::*;
use lib_core::model::drug::{FdaDeviceCode, FdaDeviceInformation};

pub fn patch_g_drugs(
	raw_xml: &[u8],
	drugs: &[DrugInformation],
	substances: &[DrugActiveSubstance],
	dosages: &[DosageInformation],
	indications: &[DrugIndication],
	characteristics: &[DrugDeviceCharacteristic],
	devices: &[FdaDeviceInformation],
	device_codes: &[FdaDeviceCode],
	assessments: &[DrugReactionAssessment],
	relatedness: &[RelatednessAssessment],
) -> Result<String> {
	patch_g_drugs_for_authority(
		raw_xml,
		drugs,
		substances,
		dosages,
		indications,
		characteristics,
		devices,
		device_codes,
		assessments,
		relatedness,
		lib_core::regulatory::RegulatoryAuthority::Ich,
	)
}

pub(crate) fn patch_g_drugs_for_authority(
	raw_xml: &[u8],
	drugs: &[DrugInformation],
	substances: &[DrugActiveSubstance],
	dosages: &[DosageInformation],
	indications: &[DrugIndication],
	characteristics: &[DrugDeviceCharacteristic],
	devices: &[FdaDeviceInformation],
	device_codes: &[FdaDeviceCode],
	assessments: &[DrugReactionAssessment],
	relatedness: &[RelatednessAssessment],
	authority: lib_core::regulatory::RegulatoryAuthority,
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
		let drug_devices: Vec<&FdaDeviceInformation> = {
			let mut rows: Vec<_> = devices
				.iter()
				.filter(|row| row.drug_id == drug.id)
				.collect();
			rows.sort_by_key(|row| row.sequence_number);
			rows
		};
		let drug_device_ids: std::collections::HashSet<_> =
			drug_devices.iter().map(|device| device.id).collect();
		let drug_device_codes: Vec<&FdaDeviceCode> = device_codes
			.iter()
			.filter(|code| drug_device_ids.contains(&code.device_id))
			.collect();
		let fragment = write_g_k_drug(
			drug,
			&subs,
			&doses,
			&inds,
			&chars,
			&drug_devices,
			&drug_device_codes,
			&drug_assessments,
			authority,
		)?;
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:primaryRole",
			&fragment,
		)?;
		let causality_fragment = write_g_k_1_causality(drug);
		if !causality_fragment.is_empty() {
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				"//hl7:adverseEventAssessment",
				&causality_fragment,
			)?;
		}
		if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda) {
			let fragment = write_fda_g_k_1_a_causality(drug);
			if !fragment.is_empty() {
				append_fragment_child(
					&mut doc,
					&parser,
					&mut xpath,
					"//hl7:adverseEventAssessment",
					&fragment,
				)?;
			}
		}
		for assessment in drug_assessments {
			let mut rows: Vec<&RelatednessAssessment> = relatedness
				.iter()
				.filter(|r| r.drug_reaction_assessment_id == assessment.id)
				.collect();
			rows.sort_by_key(|row| row.sequence_number);
			for row in rows {
				let related_fragment =
					write_g_k_9_causality(drug.id, assessment, row, authority);
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
