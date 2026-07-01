use super::*;
use crate::xml::mfds::codes::KR_C_3_1_1;

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
	let transmission_time_path = "//hl7:controlActProcess/hl7:effectiveTime";
	if let Some(transmission_date) = patch.transmission_date {
		remove_attr_first(&mut xpath, transmission_time_path, "nullFlavor");
		set_attr_first(
			&mut xpath,
			transmission_time_path,
			"value",
			&clamp_14_digit_datetime_not_future(
				&patch
					.transmission_date_value
					.filter(|v| is_14_digit_datetime(v))
					.map(|v| v.to_string())
					.or_else(|| {
						patch.transmission_date_time.map(fmt_offset_datetime)
					})
					.unwrap_or_else(|| transmission_date.to_string()),
			),
		);
	} else if let Some(null_flavor) = patch.transmission_date_null_flavor {
		remove_attr_first(&mut xpath, transmission_time_path, "value");
		set_attr_first(
			&mut xpath,
			transmission_time_path,
			"nullFlavor",
			null_flavor,
		);
	}

	// C.1.4 Date First Received
	ensure_investigation_effective_time(&mut doc, &parser, &mut xpath)?;
	let first_received_path = "//hl7:investigationEvent/hl7:effectiveTime/hl7:low";
	if let Some(date_first_received) = patch.date_first_received {
		remove_attr_first(&mut xpath, first_received_path, "nullFlavor");
		set_attr_first(
			&mut xpath,
			first_received_path,
			"value",
			&fmt_date(date_first_received),
		);
	} else if let Some(null_flavor) = patch.date_first_received_null_flavor {
		remove_attr_first(&mut xpath, first_received_path, "value");
		set_attr_first(&mut xpath, first_received_path, "nullFlavor", null_flavor);
	}

	// C.1.5 Date Most Recent
	ensure_investigation_availability_time(&mut doc, &parser, &mut xpath)?;
	let most_recent_path = "//hl7:investigationEvent/hl7:availabilityTime";
	if let Some(date_most_recent) = patch.date_most_recent {
		remove_attr_first(&mut xpath, most_recent_path, "nullFlavor");
		set_attr_first(
			&mut xpath,
			most_recent_path,
			"value",
			&fmt_date(date_most_recent),
		);
	} else if let Some(null_flavor) = patch.date_most_recent_null_flavor {
		remove_attr_first(&mut xpath, most_recent_path, "value");
		set_attr_first(&mut xpath, most_recent_path, "nullFlavor", null_flavor);
	}

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

	// C.1.6.1 Additional Documents Available
	if let Some(value) = patch.additional_documents_available {
		ensure_observation_event_component(
			&mut doc,
			&parser,
			&mut xpath,
			"1",
			"2.16.840.1.113883.3.989.2.1.1.19",
			"BL",
		)?;
		set_attr_first(
			&mut xpath,
			"//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value",
			"value",
			if value { "true" } else { "false" },
		);
	} else {
		remove_nodes(
			&mut xpath,
			"//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]",
		);
	}

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
	} else {
		remove_nodes(
			&mut xpath,
			"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']",
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
			"type",
			"CE",
		);
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
	} else {
		remove_nodes(
			&mut xpath,
			"//hl7:component/hl7:observationEvent[hl7:code[@code='C54588' and @codeSystem='2.16.840.1.113883.3.26.1.1']]",
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
	} else {
		remove_nodes(
			&mut xpath,
			"//hl7:component/hl7:observationEvent[hl7:code[@code='C156384' and @codeSystem='2.16.840.1.113883.3.26.1.1']]",
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
		"type",
		"CE",
	);
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
			"type",
			"CE",
		);
		set_attr_first(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value",
			"code",
			code,
		);
	} else {
		remove_nodes(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]",
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
	} else {
		remove_nodes(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]",
		);
	}

	// C.1.8.2 First Sender of This Case
	if let Some(code) = patch.first_sender_type {
		set_attr_first(
			&mut xpath,
			"//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']]/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code",
			"code",
			code,
		);
	} else {
		remove_nodes(
			&mut xpath,
			"//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']]",
		);
	}

	// C.3 Sender information (best-effort)
	let sender_base = "//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
	if let Some(v) = patch.sender_type {
		set_attr_first(&mut xpath, &format!("{sender_base}/hl7:code"), "code", v);
	}
	if let Some(v) = patch.sender_health_professional_type_kr1 {
		let kr1_path = &format!(
			"{sender_base}/hl7:subjectOf2/hl7:observation[hl7:code[@code='{KR_C_3_1_1}']]"
		);
		if xpath
			.findnodes(kr1_path, None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				sender_base,
				&format!(
					"<subjectOf2 typeCode=\"SUBJ\">\
						<observation classCode=\"OBS\" moodCode=\"EVN\">\
							<code code=\"{KR_C_3_1_1}\"/>\
							<value xsi:type=\"CE\"/>\
						</observation>\
					</subjectOf2>"
				),
			)?;
		}
		set_attr_first(
			&mut xpath,
			&format!(
				"{sender_base}/hl7:subjectOf2/hl7:observation[hl7:code[@code='{KR_C_3_1_1}']]/hl7:value"
			),
			"code",
			v,
		);
	}
	if let Some(v) = patch.sender_street_address {
		if xpath
			.findnodes(&format!("{sender_base}/hl7:addr"), None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				sender_base,
				"<addr/>",
			)?;
		}
		if xpath
			.findnodes(
				&format!("{sender_base}/hl7:addr/hl7:streetAddressLine"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				&format!("{sender_base}/hl7:addr"),
				"<streetAddressLine/>",
			)?;
		}
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
		if xpath
			.findnodes(&format!("{sender_base}/hl7:assignedPerson"), None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				sender_base,
				"<assignedPerson/>",
			)?;
		}
		if xpath
			.findnodes(&format!("{sender_base}/hl7:assignedPerson/hl7:name"), None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				&format!("{sender_base}/hl7:assignedPerson"),
				"<name/>",
			)?;
		}
		if xpath
			.findnodes(
				&format!("{sender_base}/hl7:assignedPerson/hl7:name/hl7:prefix"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				&format!("{sender_base}/hl7:assignedPerson/hl7:name"),
				"<prefix/>",
			)?;
		}
		set_text_first(
			&mut xpath,
			&format!("{sender_base}//hl7:assignedPerson/hl7:name/hl7:prefix"),
			v,
		);
	}
	if let Some(v) = patch.sender_person_given_name {
		if xpath
			.findnodes(&format!("{sender_base}/hl7:assignedPerson"), None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				sender_base,
				"<assignedPerson/>",
			)?;
		}
		if xpath
			.findnodes(&format!("{sender_base}/hl7:assignedPerson/hl7:name"), None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				&format!("{sender_base}/hl7:assignedPerson"),
				"<name/>",
			)?;
		}
		if xpath
			.findnodes(
				&format!("{sender_base}/hl7:assignedPerson/hl7:name/hl7:given"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				&format!("{sender_base}/hl7:assignedPerson/hl7:name"),
				"<given/>",
			)?;
		}
		set_text_first(
			&mut xpath,
			&format!("{sender_base}//hl7:assignedPerson/hl7:name/hl7:given"),
			v,
		);
	}
	if let Some(v) = patch.sender_person_middle_name {
		if xpath
			.findnodes(
				&format!("{sender_base}//hl7:assignedPerson/hl7:name/hl7:given[2]"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				&format!("{sender_base}//hl7:assignedPerson/hl7:name"),
				"<given/>",
			)?;
		}
		set_text_first(
			&mut xpath,
			&format!("{sender_base}//hl7:assignedPerson/hl7:name/hl7:given[2]"),
			v,
		);
	}
	if let Some(v) = patch.sender_person_family_name {
		if xpath
			.findnodes(&format!("{sender_base}/hl7:assignedPerson"), None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				sender_base,
				"<assignedPerson/>",
			)?;
		}
		if xpath
			.findnodes(&format!("{sender_base}/hl7:assignedPerson/hl7:name"), None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				&format!("{sender_base}/hl7:assignedPerson"),
				"<name/>",
			)?;
		}
		if xpath
			.findnodes(
				&format!("{sender_base}/hl7:assignedPerson/hl7:name/hl7:family"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				&mut doc,
				&parser,
				&mut xpath,
				&format!("{sender_base}/hl7:assignedPerson/hl7:name"),
				"<family/>",
			)?;
		}
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
