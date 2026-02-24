use crate::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use crate::model::narrative::NarrativeInformation;
use crate::model::reaction::Reaction;
use crate::model::test_result::TestResult;
use crate::xml::error::Error;
use crate::xml::export_sections::e_reaction::reaction_fragment;
use crate::xml::export_sections::f_test_result::test_result_fragment;
use crate::xml::export_sections::g_drug::drug_fragment;
use crate::xml::export_sections::h_narrative::comment_fragment;
use crate::xml::validate::{
	drug_characterization_display_name, normalize_drug_characterization,
	should_clear_null_flavor_on_value,
};
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::{Document, Node, NodeType};
use libxml::xpath::Context;
use sqlx::types::time::Date;
use sqlx::types::time::OffsetDateTime;

pub struct CSafetyReportPatch<'a> {
	pub report_unique_id: &'a str,
	pub transmission_date: Date,
	pub transmission_date_value: Option<&'a str>,
	pub transmission_date_time: Option<OffsetDateTime>,
	pub report_type: &'a str,
	pub date_first_received: Date,
	pub date_most_recent: Date,
	pub fulfil_expedited: bool,
	pub worldwide_unique_id: Option<&'a str>,
	pub local_criteria_report_type: Option<&'a str>,
	pub combination_product_indicator: Option<&'a str>,
	pub nullification_code: Option<&'a str>,
	pub nullification_reason: Option<&'a str>,
	// C.3 Sender information (best-effort; patch only when values are provided)
	pub sender_type: Option<&'a str>,
	pub sender_org_name: Option<&'a str>,
	pub sender_department: Option<&'a str>,
	pub sender_street_address: Option<&'a str>,
	pub sender_city: Option<&'a str>,
	pub sender_state: Option<&'a str>,
	pub sender_postcode: Option<&'a str>,
	pub sender_country_code: Option<&'a str>,
	pub sender_person_title: Option<&'a str>,
	pub sender_person_given_name: Option<&'a str>,
	pub sender_person_family_name: Option<&'a str>,
	pub sender_telephone: Option<&'a str>,
	pub sender_fax: Option<&'a str>,
	pub sender_email: Option<&'a str>,
}

pub struct DPatientPatch<'a> {
	pub patient_name: Option<&'a str>,
	pub sex: Option<&'a str>,
	pub birth_date: Option<Date>,
	pub age_value: Option<&'a str>,
	pub age_unit: Option<&'a str>,
	pub weight_kg: Option<&'a str>,
	pub height_cm: Option<&'a str>,
}

pub fn patch_c_safety_report(
	raw_xml: &[u8],
	patch: &CSafetyReportPatch,
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

	// C.1.1 Report Unique Identifier
	ensure_investigation_id(
		&mut doc,
		&parser,
		&mut xpath,
		"2.16.840.1.113883.3.989.2.1.3.1",
	)?;
	set_attr_first(
		&mut xpath,
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']",
		"extension",
		patch.report_unique_id,
	);

	// C.1.2 Date of Creation
	ensure_control_act_effective_time(&mut doc, &parser, &mut xpath)?;
	set_attr_first(
		&mut xpath,
		"//hl7:controlActProcess/hl7:effectiveTime",
		"value",
		&patch
			.transmission_date_value
			.filter(|v| is_14_digit_datetime(v))
			.map(|v| v.to_string())
			.or_else(|| patch.transmission_date_time.map(fmt_offset_datetime))
			.unwrap_or_else(|| fmt_date_time_fallback(patch.transmission_date)),
	);

	// C.1.4 Date First Received
	ensure_investigation_effective_time(&mut doc, &parser, &mut xpath)?;
	set_attr_first(
		&mut xpath,
		"//hl7:investigationEvent/hl7:effectiveTime/hl7:low",
		"value",
		&fmt_date(patch.date_first_received),
	);

	// C.1.5 Date Most Recent
	ensure_investigation_availability_time(&mut doc, &parser, &mut xpath)?;
	set_attr_first(
		&mut xpath,
		"//hl7:investigationEvent/hl7:availabilityTime",
		"value",
		&fmt_date(patch.date_most_recent),
	);

	// C.1.7 Expedited criteria
	ensure_observation_event_component(
		&mut doc,
		&parser,
		&mut xpath,
		"23",
		"2.16.840.1.113883.3.989.2.1.1.19",
		"BL",
	)?;
	set_attr_first(
		&mut xpath,
		"//hl7:component/hl7:observationEvent[hl7:code[@code='23' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value",
		"value",
		if patch.fulfil_expedited { "true" } else { "false" },
	);

	// C.1.8.1 Worldwide Unique Case Identification
	if let Some(worldwide_id) = patch.worldwide_unique_id {
		ensure_investigation_id(
			&mut doc,
			&parser,
			&mut xpath,
			"2.16.840.1.113883.3.989.2.1.3.2",
		)?;
		set_attr_first(
			&mut xpath,
			"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']",
			"extension",
			worldwide_id,
		);
	}

	// FDA.C.1.7.1 Local Criteria Report Type
	if let Some(code) = patch.local_criteria_report_type {
		ensure_observation_event_component(
			&mut doc,
			&parser,
			&mut xpath,
			"C54588",
			"2.16.840.1.113883.3.26.1.1",
			"CE",
		)?;
		set_attr_first(
			&mut xpath,
			"//hl7:component/hl7:observationEvent[hl7:code[@code='C54588' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value",
			"code",
			code,
		);
		clear_null_flavor_if_catalog_directive(
			&mut xpath,
			"FDA.C.1.7.1.REQUIRED",
			"//hl7:component/hl7:observationEvent[hl7:code[@code='C54588' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value",
		);
	}

	// FDA.C.1.12 Combination Product Report Indicator
	if let Some(value) = patch.combination_product_indicator {
		let normalized = normalize_bl_value(value).unwrap_or("false");
		ensure_observation_event_component(
			&mut doc,
			&parser,
			&mut xpath,
			"C156384",
			"2.16.840.1.113883.3.26.1.1",
			"BL",
		)?;
		set_attr_first(
			&mut xpath,
			"//hl7:component/hl7:observationEvent[hl7:code[@code='C156384' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value",
			"value",
			normalized,
		);
		clear_null_flavor_if_catalog_directive(
			&mut xpath,
			"FDA.C.1.12.REQUIRED",
			"//hl7:component/hl7:observationEvent[hl7:code[@code='C156384' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value",
		);
	}

	// C.1.3 Type of Report
	// Keep investigationCharacteristic insertion after component insertions so
	// investigationEvent children remain in schema order.
	ensure_investigation_characteristic(
		&mut doc,
		&parser,
		&mut xpath,
		"1",
		"2.16.840.1.113883.3.989.2.1.1.23",
		Some("2.16.840.1.113883.3.989.2.1.1.2"),
	)?;
	set_attr_first(
		&mut xpath,
		"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value",
		"code",
		patch.report_type,
	);

	// C.1.11.1 Nullification/Amendment Code
	if let Some(code) = patch.nullification_code {
		ensure_investigation_characteristic(
			&mut doc,
			&parser,
			&mut xpath,
			"3",
			"2.16.840.1.113883.3.989.2.1.1.23",
			None,
		)?;
		set_attr_first(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value",
			"code",
			code,
		);
	}

	// C.1.11.2 Nullification/Amendment Reason
	if let Some(reason) = patch.nullification_reason {
		ensure_investigation_characteristic(
			&mut doc,
			&parser,
			&mut xpath,
			"4",
			"2.16.840.1.113883.3.989.2.1.1.23",
			None,
		)?;
		set_text_first(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/hl7:originalText",
			reason,
		);
	}

	// C.3 Sender information (best-effort)
	let sender_base = "//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
	if let Some(v) = patch.sender_type {
		set_attr_first(&mut xpath, &format!("{sender_base}/hl7:code"), "code", v);
	}
	if let Some(v) = patch.sender_street_address {
		set_text_first(
			&mut xpath,
			&format!("{sender_base}/hl7:addr/hl7:streetAddressLine"),
			v,
		);
	}
	if let Some(v) = patch.sender_city {
		set_text_first(&mut xpath, &format!("{sender_base}/hl7:addr/hl7:city"), v);
	}
	if let Some(v) = patch.sender_state {
		set_text_first(&mut xpath, &format!("{sender_base}/hl7:addr/hl7:state"), v);
	}
	if let Some(v) = patch.sender_postcode {
		set_text_first(
			&mut xpath,
			&format!("{sender_base}/hl7:addr/hl7:postalCode"),
			v,
		);
	}
	if let Some(v) = patch.sender_country_code {
		set_attr_first(
			&mut xpath,
			&format!("{sender_base}//hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code"),
			"code",
			v,
		);
	}
	if let Some(v) = patch.sender_person_title {
		set_text_first(
			&mut xpath,
			&format!("{sender_base}//hl7:assignedPerson/hl7:name/hl7:prefix"),
			v,
		);
	}
	if let Some(v) = patch.sender_person_given_name {
		set_text_first(
			&mut xpath,
			&format!("{sender_base}//hl7:assignedPerson/hl7:name/hl7:given"),
			v,
		);
	}
	if let Some(v) = patch.sender_person_family_name {
		set_text_first(
			&mut xpath,
			&format!("{sender_base}//hl7:assignedPerson/hl7:name/hl7:family"),
			v,
		);
	}
	if let Some(v) = patch.sender_department {
		set_text_first(
			&mut xpath,
			&format!("{sender_base}/hl7:representedOrganization/hl7:name"),
			v,
		);
	}
	if let Some(v) = patch.sender_org_name {
		set_text_first(
			&mut xpath,
			&format!("{sender_base}/hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name"),
			v,
		);
	}
	if let Some(v) = patch.sender_telephone {
		let value = if v.contains(':') {
			v.to_string()
		} else {
			format!("tel:{v}")
		};
		set_attr_first(
			&mut xpath,
			&format!("{sender_base}/hl7:telecom[starts-with(@value,'tel:')]"),
			"value",
			&value,
		);
	}
	if let Some(v) = patch.sender_fax {
		let value = if v.contains(':') {
			v.to_string()
		} else {
			format!("fax:{v}")
		};
		set_attr_first(
			&mut xpath,
			&format!("{sender_base}/hl7:telecom[starts-with(@value,'fax:')]"),
			"value",
			&value,
		);
	}
	if let Some(v) = patch.sender_email {
		let value = if v.contains(':') {
			v.to_string()
		} else {
			format!("mailto:{v}")
		};
		set_attr_first(
			&mut xpath,
			&format!("{sender_base}/hl7:telecom[starts-with(@value,'mailto:')]"),
			"value",
			&value,
		);
	}

	Ok(doc.to_string())
}

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

pub fn patch_e_reactions(raw_xml: &[u8], reactions: &[Reaction]) -> Result<String> {
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
		"//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='29' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]",
	);
	for reaction in reactions {
		let fragment = reaction_fragment(reaction)?;
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:primaryRole",
			&fragment,
		)?;
	}

	Ok(doc.to_string())
}

pub fn patch_f_test_results(raw_xml: &[u8], tests: &[TestResult]) -> Result<String> {
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
		"//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]",
	);
	for test in tests {
		let fragment = test_result_fragment(test);
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:primaryRole",
			&fragment,
		)?;
	}

	Ok(doc.to_string())
}

pub fn patch_g_drugs(
	raw_xml: &[u8],
	drugs: &[DrugInformation],
	substances: &[DrugActiveSubstance],
	dosages: &[DosageInformation],
	indications: &[DrugIndication],
	characteristics: &[DrugDeviceCharacteristic],
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
	}

	Ok(doc.to_string())
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
		"<component typeCode=\"COMP\">\
			<causalityAssessment classCode=\"OBS\" moodCode=\"EVN\">\
				<code code=\"20\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"interventionCharacterization\"/>\
				<value xsi:type=\"CE\" code=\"{role_code}\" displayName=\"{display}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.13\"/>\
				<subject2 typeCode=\"SUBJ\">\
					<productUseReference classCode=\"SBADM\" moodCode=\"EVN\">\
						<id root=\"{}\"/>\
					</productUseReference>\
				</subject2>\
			</causalityAssessment>\
		</component>",
		xml_escape(&drug.id.to_string())
	))
}

fn xml_escape(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}

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

fn ensure_investigation_id(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	root: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='{root}']"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent",
		&format!("<id root=\"{root}\"/>"),
	)
}

fn ensure_primary_role(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if xpath
		.findnodes("//hl7:primaryRole/hl7:player1", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let fragment = "\
		<subject1 typeCode=\"SBJ\">\
			<primaryRole classCode=\"PAT\">\
				<player1 classCode=\"PSN\" determinerCode=\"INSTANCE\">\
					<name/>\
					<administrativeGenderCode code=\"0\" codeSystem=\"1.0.5218\"/>\
					<birthTime/>\
				</player1>\
			</primaryRole>\
		</subject1>";
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:adverseEventAssessment",
		fragment,
	)
}

fn ensure_subject_observation(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	code: &str,
	code_system: &str,
	value_type: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='{code}' and @codeSystem='{code_system}']]"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let fragment = format!(
		"<subjectOf2 typeCode=\"SBJ\">\
			<observation classCode=\"OBS\" moodCode=\"EVN\">\
				<code code=\"{code}\" codeSystem=\"{code_system}\"/>\
				<value xsi:type=\"{value_type}\"/>\
			</observation>\
		</subjectOf2>"
	);
	append_fragment_child(doc, parser, xpath, "//hl7:primaryRole", &fragment)
}

fn ensure_control_act_effective_time(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if xpath
		.findnodes("//hl7:controlActProcess/hl7:effectiveTime", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:controlActProcess",
		"<effectiveTime/>",
	)
}

fn ensure_investigation_effective_time(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if xpath
		.findnodes("//hl7:investigationEvent/hl7:effectiveTime/hl7:low", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	if xpath
		.findnodes("//hl7:investigationEvent/hl7:effectiveTime", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent/hl7:effectiveTime",
			"<low/>",
		)
	} else {
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent",
			"<effectiveTime><low/></effectiveTime>",
		)
	}
}

fn ensure_investigation_availability_time(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if xpath
		.findnodes("//hl7:investigationEvent/hl7:availabilityTime", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:investigationEvent",
		"<availabilityTime/>",
	)
}

fn ensure_investigation_text(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if xpath
		.findnodes("//hl7:investigationEvent/hl7:text", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	append_fragment_child(doc, parser, xpath, "//hl7:investigationEvent", "<text/>")
}

fn ensure_investigation_characteristic(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	code: &str,
	code_system: &str,
	value_code_system: Option<&str>,
) -> Result<()> {
	let path = format!(
		"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='{code}' and @codeSystem='{code_system}']]"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let value_cs = value_code_system
		.map(|cs| format!(" codeSystem=\"{cs}\""))
		.unwrap_or_default();
	let fragment = format!(
		"<subjectOf2 typeCode=\"SUBJ\">\
			<investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\">\
				<code code=\"{code}\" codeSystem=\"{code_system}\"/>\
				<value xsi:type=\"CE\"{value_cs}><originalText/></value>\
			</investigationCharacteristic>\
		</subjectOf2>"
	);
	append_fragment_child(doc, parser, xpath, "//hl7:investigationEvent", &fragment)
}

fn ensure_observation_event_component(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	code: &str,
	code_system: &str,
	value_type: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:component/hl7:observationEvent[hl7:code[@code='{code}' and @codeSystem='{code_system}']]"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let fragment = format!(
		"<component typeCode=\"COMP\">\
			<observationEvent classCode=\"OBS\" moodCode=\"EVN\">\
				<code code=\"{code}\" codeSystem=\"{code_system}\"/>\
				<value xsi:type=\"{value_type}\"/>\
			</observationEvent>\
		</component>"
	);
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:investigationEvent",
		&fragment,
	)?;
	reorder_investigation_event_children(xpath);
	Ok(())
}

fn reorder_investigation_event_children(xpath: &mut Context) {
	if let Ok(subject_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:subjectOf2", None)
	{
		for mut node in subject_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
}

fn append_fragment_child(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	parent_path: &str,
	fragment: &str,
) -> Result<()> {
	let mut parent = xpath
		.findnodes(parent_path, None)
		.map_err(|_| Error::InvalidXml {
			message: format!("Failed to find nodes for path {parent_path}"),
			line: None,
			column: None,
		})?
		.into_iter()
		.next()
		.ok_or(Error::InvalidXml {
			message: format!("Failed to find nodes for path {parent_path}"),
			line: None,
			column: None,
		})?;

	let mut node = node_from_fragment(doc, parser, fragment)?;
	parent
		.add_child(&mut node)
		.map_err(|err| Error::InvalidXml {
			message: format!("Failed to append fragment: {err}"),
			line: None,
			column: None,
		})?;
	Ok(())
}

fn remove_nodes(xpath: &mut Context, path: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		for mut node in nodes {
			node.unlink_node();
		}
	}
}

fn node_from_fragment(
	doc: &mut Document,
	parser: &Parser,
	fragment: &str,
) -> Result<Node> {
	let fragment = wrap_fragment(fragment, "urn:hl7-org:v3");
	let frag_doc =
		parser
			.parse_string(&fragment)
			.map_err(|err| Error::InvalidXml {
				message: format!("XML parse error: {err}"),
				line: None,
				column: None,
			})?;
	let root = frag_doc.get_root_element().ok_or(Error::InvalidXml {
		message: "Failed to get fragment root".to_string(),
		line: None,
		column: None,
	})?;
	let mut child = root
		.get_child_nodes()
		.into_iter()
		.find(|n| n.get_type() == Some(NodeType::ElementNode))
		.ok_or(Error::InvalidXml {
			message: "Failed to get fragment child".to_string(),
			line: None,
			column: None,
		})?;
	child.unlink_node();
	doc.import_node(&mut child).map_err(|_| Error::InvalidXml {
		message: "Failed to import cloned node".to_string(),
		line: None,
		column: None,
	})
}

fn wrap_fragment(fragment: &str, ns: &str) -> String {
	format!(
		"<wrapper xmlns=\"{ns}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">{fragment}</wrapper>"
	)
}

fn set_attr_first(xpath: &mut Context, path: &str, attr: &str, value: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		if let Some(mut node) = nodes.into_iter().next() {
			let _ = node.set_attribute(attr, value);
		}
	}
}

fn set_text_first(xpath: &mut Context, path: &str, value: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		if let Some(mut node) = nodes.into_iter().next() {
			let _ = node.set_content(value);
		}
	}
}

fn remove_attr_first(xpath: &mut Context, path: &str, attr: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		if let Some(mut node) = nodes.into_iter().next() {
			let _ = node.remove_attribute(attr);
		}
	}
}

fn clear_null_flavor_if_catalog_directive(
	xpath: &mut Context,
	rule_code: &str,
	path: &str,
) {
	if should_clear_null_flavor_on_value(rule_code) {
		remove_attr_first(xpath, path, "nullFlavor");
	}
}

fn fmt_date(date: Date) -> String {
	let year = date.year();
	let month: u8 = date.month().into();
	let day = date.day();
	format!("{:04}{:02}{:02}", year, month, day)
}

fn fmt_date_time_fallback(date: Date) -> String {
	format!("{}000000", fmt_date(date))
}

fn fmt_offset_datetime(dt: OffsetDateTime) -> String {
	format!(
		"{:04}{:02}{:02}{:02}{:02}{:02}",
		dt.year(),
		u8::from(dt.month()),
		dt.day(),
		dt.hour(),
		dt.minute(),
		dt.second()
	)
}

fn is_14_digit_datetime(value: &str) -> bool {
	value.len() == 14 && value.chars().all(|c| c.is_ascii_digit())
}

fn normalize_bl_value(value: &str) -> Option<&'static str> {
	match value.trim().to_ascii_lowercase().as_str() {
		"true" | "1" | "y" | "yes" => Some("true"),
		"false" | "0" | "2" | "n" | "no" => Some("false"),
		_ => None,
	}
}
