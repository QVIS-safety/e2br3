use super::*;

pub fn patch_d_patient(raw_xml: &[u8], patch: &DPatientPatch) -> Result<String> {
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

	if let Some(name) = patch.patient_name {
		set_text_first(&mut xpath, "//hl7:primaryRole/hl7:player1/hl7:name", name);
	}

	if let Some(sex) = patch.sex {
		set_attr_first(
			&mut xpath,
			"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode",
			"code",
			sex,
		);
	}

	if let Some(birth_date) = patch.birth_date {
		set_attr_first(
			&mut xpath,
			"//hl7:primaryRole/hl7:player1/hl7:birthTime",
			"value",
			&fmt_date(birth_date),
		);
	}

	if let Some(age) = patch.age_value {
		ensure_subject_observation(
			&mut doc,
			&parser,
			&mut xpath,
			"3",
			"2.16.840.1.113883.3.989.2.1.1.19",
			"PQ",
		)?;
		set_attr_first(
			&mut xpath,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value",
			"value",
			age,
		);
		if let Some(unit) = patch.age_unit {
			set_attr_first(
				&mut xpath,
				"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value",
				"unit",
				unit,
			);
		}
	}

	if let Some(weight) = patch.weight_kg {
		ensure_subject_observation(
			&mut doc,
			&parser,
			&mut xpath,
			"7",
			"2.16.840.1.113883.3.989.2.1.1.19",
			"PQ",
		)?;
		set_attr_first(
			&mut xpath,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='7' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value",
			"value",
			weight,
		);
	}

	if let Some(height) = patch.height_cm {
		ensure_subject_observation(
			&mut doc,
			&parser,
			&mut xpath,
			"17",
			"2.16.840.1.113883.3.989.2.1.1.19",
			"PQ",
		)?;
		set_attr_first(
			&mut xpath,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='17' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value",
			"value",
			height,
		);
	}

	Ok(doc.to_string())
}

