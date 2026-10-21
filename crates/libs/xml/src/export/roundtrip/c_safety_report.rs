use super::*;
use crate::export_utils::{fmt_date_lexeme, set_xsi_type_first};

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

	write_c_1_1(&mut doc, &parser, &mut xpath, patch.report_unique_id)?;
	write_c_1_2(&mut doc, &parser, &mut xpath, patch)?;
	write_c_1_4(&mut doc, &parser, &mut xpath, patch.date_first_received)?;
	write_c_1_5(&mut doc, &parser, &mut xpath, patch.date_most_recent)?;

	write_c_1_7(
		&mut doc,
		&parser,
		&mut xpath,
		patch.fulfil_expedited,
		patch.fulfil_expedited_null_flavor,
	)?;
	write_c_1_6_1(
		&mut doc,
		&parser,
		&mut xpath,
		patch.additional_documents_available,
	)?;
	write_c_1_8_1(&mut doc, &parser, &mut xpath, patch.worldwide_unique_id)?;

	write_fda_c_1_7_1(
		&mut doc,
		&parser,
		&mut xpath,
		patch.local_criteria_report_type,
	)?;
	write_fda_c_1_12(
		&mut doc,
		&parser,
		&mut xpath,
		patch.combination_product_indicator,
		patch.combination_product_indicator_null_flavor,
	)?;
	write_c_1_3(&mut doc, &parser, &mut xpath, patch.report_type)?;
	write_c_1_9_1(&mut doc, &parser, &mut xpath, patch)?;

	write_c_1_11_1(&mut doc, &parser, &mut xpath, patch.nullification_code)?;
	write_c_1_11_2(&mut doc, &parser, &mut xpath, patch.nullification_reason)?;

	write_c_1_8_2(&mut doc, &parser, &mut xpath, patch.first_sender_type)?;

	// C.3 Sender information
	let sender_base = "//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
	ensure_c_3_nodes(&mut doc, &parser, &mut xpath, sender_base, patch)?;
	if let Some(v) = patch.sender_type {
		write_c_3_1(&mut xpath, sender_base, v);
	}
	write_c_3_1_kr_1(
		&mut doc,
		&parser,
		&mut xpath,
		sender_base,
		patch.sender_health_professional_type_kr1,
	)?;

	if let Some(v) = patch.sender_street_address {
		write_c_3_4_1(&mut xpath, sender_base, v);
	}
	if let Some(v) = patch.sender_city {
		write_c_3_4_2(&mut xpath, sender_base, v);
	}
	if let Some(v) = patch.sender_state {
		write_c_3_4_3(&mut xpath, sender_base, v);
	}
	if let Some(v) = patch.sender_postcode {
		write_c_3_4_4(&mut xpath, sender_base, v);
	}
	if let Some(v) = patch.sender_country_code {
		write_c_3_4_5(&mut xpath, sender_base, v);
	}
	if patch.sender_person_title.is_some()
		|| patch.sender_person_title_null_flavor.is_some()
	{
		write_c_3_3_2(
			&mut xpath,
			sender_base,
			patch.sender_person_title,
			patch.sender_person_title_null_flavor,
		);
	}
	if patch.sender_person_given_name.is_some()
		|| patch.sender_person_given_name_null_flavor.is_some()
	{
		write_c_3_3_3(
			&mut xpath,
			sender_base,
			patch.sender_person_given_name,
			patch.sender_person_given_name_null_flavor,
		);
	}
	if patch.sender_person_middle_name.is_some()
		|| patch.sender_person_middle_name_null_flavor.is_some()
	{
		write_c_3_3_4(
			&mut xpath,
			sender_base,
			patch.sender_person_middle_name,
			patch.sender_person_middle_name_null_flavor,
		);
	}
	if patch.sender_person_family_name.is_some()
		|| patch.sender_person_family_name_null_flavor.is_some()
	{
		write_c_3_3_5(
			&mut xpath,
			sender_base,
			patch.sender_person_family_name,
			patch.sender_person_family_name_null_flavor,
		);
	}
	if let Some(v) = patch.sender_department {
		write_c_3_3_1(&mut xpath, sender_base, v);
	}
	if let Some(v) = patch.sender_org_name {
		write_c_3_2(&mut xpath, sender_base, v);
	}
	write_c_3_4_6(&mut xpath, sender_base, patch.sender_telephone);
	write_c_3_4_7(&mut xpath, sender_base, patch.sender_fax);
	write_c_3_4_8(&mut xpath, sender_base, patch.sender_email);

	Ok(doc.to_string())
}

fn ensure_c_3_nodes(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	base: &str,
	patch: &CSafetyReportPatch<'_>,
) -> Result<()> {
	let value =
		|value: Option<&str>| value.is_some_and(|value| !value.trim().is_empty());
	let present = |value_, null_flavor| value(value_) || value(null_flavor);
	let mut ensure = |parent: &str, path: &str, fragment: &str| {
		if xpath
			.findnodes(path, None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(doc, parser, xpath, parent, fragment)?;
		}
		Ok::<_, Error>(())
	};

	if value(patch.sender_type) {
		ensure(
			base,
			&format!("{base}/hl7:code"),
			"<code codeSystem=\"2.16.840.1.113883.3.989.2.1.1.7\"/>",
		)?;
	}

	if value(patch.sender_department) || value(patch.sender_org_name) {
		ensure(
			base,
			&format!("{base}/hl7:representedOrganization"),
			"<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"/>",
		)?;
	}
	if value(patch.sender_department) {
		ensure(
			&format!("{base}/hl7:representedOrganization"),
			&format!("{base}/hl7:representedOrganization/hl7:name"),
			"<name/>",
		)?;
	}
	if value(patch.sender_org_name) {
		let organization = format!("{base}/hl7:representedOrganization");
		ensure(
			&organization,
			&format!("{organization}/hl7:assignedEntity"),
			"<assignedEntity classCode=\"ASSIGNED\"/>",
		)?;
		let assigned = format!("{organization}/hl7:assignedEntity");
		ensure(
			&assigned,
			&format!("{assigned}/hl7:representedOrganization"),
			"<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"/>",
		)?;
		let nested = format!("{assigned}/hl7:representedOrganization");
		ensure(&nested, &format!("{nested}/hl7:name"), "<name/>")?;
	}

	if [
		patch.sender_street_address,
		patch.sender_city,
		patch.sender_state,
		patch.sender_postcode,
	]
	.into_iter()
	.any(value)
	{
		ensure(base, &format!("{base}/hl7:addr"), "<addr/>")?;
		for (field, fragment) in [
			("streetAddressLine", "<streetAddressLine/>"),
			("city", "<city/>"),
			("state", "<state/>"),
			("postalCode", "<postalCode/>"),
		] {
			let field_value = match field {
				"streetAddressLine" => patch.sender_street_address,
				"city" => patch.sender_city,
				"state" => patch.sender_state,
				"postalCode" => patch.sender_postcode,
				_ => None,
			};
			if value(field_value) {
				ensure(
					&format!("{base}/hl7:addr"),
					&format!("{base}/hl7:addr/hl7:{field}"),
					fragment,
				)?;
			}
		}
	}

	if [
		patch.sender_health_professional_type_kr1,
		patch.sender_person_title,
		patch.sender_person_title_null_flavor,
		patch.sender_person_given_name,
		patch.sender_person_given_name_null_flavor,
		patch.sender_person_middle_name,
		patch.sender_person_middle_name_null_flavor,
		patch.sender_person_family_name,
		patch.sender_person_family_name_null_flavor,
		patch.sender_country_code,
	]
	.into_iter()
	.any(value)
	{
		ensure(
			base,
			&format!("{base}/hl7:assignedPerson"),
			"<assignedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"/>",
		)?;
	}
	if [
		patch.sender_person_title,
		patch.sender_person_title_null_flavor,
		patch.sender_person_given_name,
		patch.sender_person_given_name_null_flavor,
		patch.sender_person_middle_name,
		patch.sender_person_middle_name_null_flavor,
		patch.sender_person_family_name,
		patch.sender_person_family_name_null_flavor,
	]
	.into_iter()
	.any(value)
	{
		ensure(
			&format!("{base}/hl7:assignedPerson"),
			&format!("{base}/hl7:assignedPerson/hl7:name"),
			"<name/>",
		)?;
		for (field, fragment) in [
			("prefix", "<prefix/>"),
			("given[1]", "<given/>"),
			("family", "<family/>"),
		] {
			let field_value = match field {
				"prefix" => (
					patch.sender_person_title,
					patch.sender_person_title_null_flavor,
				),
				"given[1]" => (
					patch.sender_person_given_name,
					patch.sender_person_given_name_null_flavor,
				),
				"family" => (
					patch.sender_person_family_name,
					patch.sender_person_family_name_null_flavor,
				),
				_ => (None, None),
			};
			if present(field_value.0, field_value.1) {
				ensure(
					&format!("{base}/hl7:assignedPerson/hl7:name"),
					&format!("{base}/hl7:assignedPerson/hl7:name/hl7:{field}"),
					fragment,
				)?;
			}
		}
		if present(
			patch.sender_person_middle_name,
			patch.sender_person_middle_name_null_flavor,
		) {
			ensure(
				&format!("{base}/hl7:assignedPerson/hl7:name"),
				&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[2]"),
				"<given/>",
			)?;
		}
	}

	if value(patch.sender_country_code) {
		let person = format!("{base}/hl7:assignedPerson");
		ensure(
			&person,
			&format!("{person}/hl7:asLocatedEntity"),
			"<asLocatedEntity classCode=\"LOCE\"/>",
		)?;
		let located = format!("{person}/hl7:asLocatedEntity");
		ensure(
			&located,
			&format!("{located}/hl7:location"),
			"<location classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"/>",
		)?;
		let location = format!("{located}/hl7:location");
		ensure(
			&location,
			&format!("{location}/hl7:code"),
			"<code codeSystem=\"1.0.3166.1.2.2\"/>",
		)?;
	}

	for (prefix, field_value) in [
		("tel:", patch.sender_telephone),
		("fax:", patch.sender_fax),
		("mailto:", patch.sender_email),
	] {
		if normalized_c_3_4_telecom(prefix, field_value).is_some() {
			ensure(
				base,
				&format!("{base}/hl7:telecom[starts-with(@value,'{prefix}')]"),
				&format!("<telecom value=\"{prefix}\"/>"),
			)?;
		}
	}

	Ok(())
}

/// e2b:C.1.1
fn write_c_1_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	report_unique_id: &str,
) -> Result<()> {
	ensure_investigation_id(doc, parser, xpath, "2.16.840.1.113883.3.989.2.1.3.1")?;
	set_attr_first(
		xpath,
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']",
		"extension",
		report_unique_id,
	);
	Ok(())
}

/// e2b:C.1.2
fn write_c_1_2(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	patch: &CSafetyReportPatch<'_>,
) -> Result<()> {
	ensure_control_act_effective_time(doc, parser, xpath)?;
	let path = "//hl7:controlActProcess/hl7:effectiveTime";
	if let Some(transmission_date) = patch.transmission_date {
		remove_attr_first(xpath, path, "nullFlavor");
		set_attr_first(xpath, path, "value", transmission_date);
	}
	Ok(())
}

/// e2b:C.1.4
fn write_c_1_4(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	date_first_received: Option<&str>,
) -> Result<()> {
	ensure_investigation_effective_time(doc, parser, xpath)?;
	let path = "//hl7:investigationEvent/hl7:effectiveTime/hl7:low";
	if let Some(value) = date_first_received {
		remove_attr_first(xpath, path, "nullFlavor");
		set_attr_first(xpath, path, "value", &fmt_date_lexeme(value));
	}
	Ok(())
}

/// e2b:C.1.5
fn write_c_1_5(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	date_most_recent: Option<&str>,
) -> Result<()> {
	ensure_investigation_availability_time(doc, parser, xpath)?;
	let path = "//hl7:investigationEvent/hl7:availabilityTime";
	if let Some(value) = date_most_recent {
		remove_attr_first(xpath, path, "nullFlavor");
		set_attr_first(xpath, path, "value", &fmt_date_lexeme(value));
	}
	Ok(())
}

/// e2b:C.1.6.1
fn write_c_1_6_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	additional_documents_available: Option<bool>,
) -> Result<()> {
	let path = "//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]";
	if let Some(value) = additional_documents_available {
		ensure_observation_event_component(
			doc,
			parser,
			xpath,
			"1",
			"2.16.840.1.113883.3.989.2.1.1.19",
			"BL",
		)?;
		set_attr_first(
			xpath,
			&format!("{path}/hl7:value"),
			"value",
			if value { "true" } else { "false" },
		);
	} else {
		remove_nodes(xpath, path);
	}
	Ok(())
}

/// e2b:C.1.7
fn write_c_1_7(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	fulfil_expedited: Option<bool>,
	null_flavor: Option<&str>,
) -> Result<()> {
	ensure_observation_event_component(
		doc,
		parser,
		xpath,
		"23",
		"2.16.840.1.113883.3.989.2.1.1.19",
		"BL",
	)?;
	let path = "//hl7:component/hl7:observationEvent[hl7:code[@code='23' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value";
	if let Some(value) = fulfil_expedited {
		remove_attr_first(xpath, path, "nullFlavor");
		set_attr_first(xpath, path, "value", if value { "true" } else { "false" });
	} else if let Some(null_flavor) = null_flavor {
		remove_attr_first(xpath, path, "value");
		set_attr_first(xpath, path, "nullFlavor", null_flavor);
	}
	Ok(())
}

/// e2b:C.1.8.1
fn write_c_1_8_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	worldwide_id: Option<&str>,
) -> Result<()> {
	let path = "//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']";
	if let Some(value) = worldwide_id {
		ensure_investigation_id(
			doc,
			parser,
			xpath,
			"2.16.840.1.113883.3.989.2.1.3.2",
		)?;
		set_attr_first(xpath, path, "extension", value);
	} else {
		remove_nodes(xpath, path);
	}
	Ok(())
}

/// e2b:C.1.3
fn write_c_1_3(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	report_type: &str,
) -> Result<()> {
	// Run after component writers to preserve investigationEvent schema order.
	ensure_investigation_characteristic(
		doc,
		parser,
		xpath,
		"1",
		"2.16.840.1.113883.3.989.2.1.1.23",
		Some("2.16.840.1.113883.3.989.2.1.1.2"),
	)?;
	let path = "//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value";
	set_attr_first(xpath, path, "type", "CE");
	set_attr_first(xpath, path, "code", report_type);
	remove_nodes(xpath, &format!("{path}/hl7:originalText"));
	Ok(())
}

/// e2b:C.1.9.1
fn write_c_1_9_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	patch: &CSafetyReportPatch,
) -> Result<()> {
	let characteristic = "//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]";
	if patch.other_case_identifiers_exist.is_none()
		&& patch.other_case_identifiers_exist_null_flavor.is_none()
	{
		remove_nodes(xpath, characteristic);
		return Ok(());
	}
	ensure_investigation_characteristic(
		doc,
		parser,
		xpath,
		"2",
		"2.16.840.1.113883.3.989.2.1.1.23",
		None,
	)?;
	let path = &format!("{characteristic}/hl7:value");
	remove_attr_first(xpath, path, "type");
	for attr in ["code", "codeSystem", "codeSystemVersion", "displayName"] {
		remove_attr_first(xpath, path, attr);
	}
	set_xsi_type_first(xpath, path, "BL")?;
	remove_nodes(xpath, &format!("{path}/hl7:originalText"));
	if let Some(value) = patch.other_case_identifiers_exist {
		remove_attr_first(xpath, path, "nullFlavor");
		set_attr_first(xpath, path, "value", if value { "true" } else { "false" });
	} else if let Some(null_flavor) = patch.other_case_identifiers_exist_null_flavor
	{
		remove_attr_first(xpath, path, "value");
		set_attr_first(xpath, path, "nullFlavor", null_flavor);
	}
	Ok(())
}

/// e2b:C.1.8.2
fn write_c_1_8_2(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	first_sender_type: Option<&str>,
) -> Result<()> {
	let relationship =
		"//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']]";
	if let Some(value) = first_sender_type {
		if xpath
			.findnodes(relationship, None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				"//hl7:investigationEvent",
				"<outboundRelationship typeCode=\"SPRT\"><relatedInvestigation classCode=\"INVSTG\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.22\"/><subjectOf2 typeCode=\"SUBJ\"><controlActEvent classCode=\"CACT\" moodCode=\"EVN\"><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code codeSystem=\"2.16.840.1.113883.3.989.2.1.1.3\"/></assignedEntity></author></controlActEvent></subjectOf2></relatedInvestigation></outboundRelationship>",
			)?;
		}
		reorder_investigation_event_children(xpath);
		set_attr_first(
			xpath,
			&format!("{relationship}/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code"),
			"code",
			value,
		);
	} else {
		remove_nodes(xpath, relationship);
	}
	Ok(())
}

/// e2b:C.1.11.1
fn write_c_1_11_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	nullification_code: Option<&str>,
) -> Result<()> {
	let characteristic = "//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]";
	if let Some(value) = nullification_code {
		ensure_investigation_characteristic(
			doc,
			parser,
			xpath,
			"3",
			"2.16.840.1.113883.3.989.2.1.1.23",
			None,
		)?;
		let path = format!("{characteristic}/hl7:value");
		set_attr_first(xpath, &path, "type", "CE");
		set_attr_first(xpath, &path, "code", value);
	} else {
		remove_nodes(xpath, characteristic);
	}
	Ok(())
}

/// e2b:C.1.11.2
fn write_c_1_11_2(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	nullification_reason: Option<&str>,
) -> Result<()> {
	let characteristic = "//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]";
	if let Some(value) = nullification_reason {
		ensure_investigation_characteristic(
			doc,
			parser,
			xpath,
			"4",
			"2.16.840.1.113883.3.989.2.1.1.23",
			None,
		)?;
		set_text_first(
			xpath,
			&format!("{characteristic}/hl7:value/hl7:originalText"),
			value,
		);
	} else {
		remove_nodes(xpath, characteristic);
	}
	Ok(())
}

/// e2b:FDA.C.1.7.1
fn write_fda_c_1_7_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	local_criteria_report_type: Option<&str>,
) -> Result<()> {
	let component = "//hl7:component/hl7:observationEvent[hl7:code[@code='C54588' and @codeSystem='2.16.840.1.113883.3.26.1.1']]";
	if let Some(value) = local_criteria_report_type {
		ensure_observation_event_component(
			doc,
			parser,
			xpath,
			"C54588",
			"2.16.840.1.113883.3.26.1.1",
			"CE",
		)?;
		let path = format!("{component}/hl7:value");
		remove_attr_first(xpath, &path, "type");
		set_xsi_type_first(xpath, &path, "CE")?;
		set_attr_first(xpath, &path, "code", value);
		set_attr_first(
			xpath,
			&path,
			"codeSystem",
			"2.16.840.1.113883.3.989.5.1.2.2.1.1.1",
		);
		clear_null_flavor_if_export_policy(xpath, "FDA.C.1.7.1.REQUIRED", &path);
	} else {
		remove_nodes(xpath, component);
	}
	Ok(())
}

/// e2b:FDA.C.1.12
fn write_fda_c_1_12(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	combination_product_indicator: Option<&str>,
	combination_product_indicator_null_flavor: Option<&str>,
) -> Result<()> {
	let component = "//hl7:component/hl7:observationEvent[hl7:code[@code='C156384' and @codeSystem='2.16.840.1.113883.3.26.1.1']]";
	if combination_product_indicator.is_none()
		&& combination_product_indicator_null_flavor.is_none()
	{
		remove_nodes(xpath, component);
		return Ok(());
	}
	ensure_observation_event_component(
		doc,
		parser,
		xpath,
		"C156384",
		"2.16.840.1.113883.3.26.1.1",
		"BL",
	)?;
	let path = format!("{component}/hl7:value");
	if let Some(value) = combination_product_indicator {
		set_attr_first(
			xpath,
			&path,
			"value",
			normalize_bl_value(value).unwrap_or("false"),
		);
		clear_null_flavor_if_export_policy(xpath, "FDA.C.1.12.REQUIRED", &path);
	} else if let Some(null_flavor) = combination_product_indicator_null_flavor {
		remove_attr_first(xpath, &path, "value");
		set_attr_first(xpath, &path, "nullFlavor", null_flavor);
	}
	Ok(())
}

/// e2b:C.3.1
fn write_c_3_1(xpath: &mut Context, base: &str, value: &str) {
	set_attr_first(xpath, &format!("{base}/hl7:code"), "code", value);
}

/// e2b:C.3.1.KR.1
fn write_c_3_1_kr_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	base: &str,
	value: Option<&str>,
) -> Result<()> {
	crate::export_utils::write_qualification_code(
		doc,
		parser,
		xpath,
		base,
		"2.16.840.1.113883.3.989.5.1.10.1.2",
		value,
	)
}

/// e2b:C.3.2
fn write_c_3_2(xpath: &mut Context, base: &str, value: &str) {
	set_text_first(
		xpath,
		&format!("{base}/hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name"),
		value,
	);
}

/// e2b:C.3.3.1
fn write_c_3_3_1(xpath: &mut Context, base: &str, value: &str) {
	set_text_first(
		xpath,
		&format!("{base}/hl7:representedOrganization/hl7:name"),
		value,
	);
}

/// e2b:C.3.3.2
fn write_c_3_3_2(
	xpath: &mut Context,
	base: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}//hl7:assignedPerson/hl7:name/hl7:prefix"),
		value,
		null_flavor,
	);
}

/// e2b:C.3.3.3
fn write_c_3_3_3(
	xpath: &mut Context,
	base: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}//hl7:assignedPerson/hl7:name/hl7:given"),
		value,
		null_flavor,
	);
}

/// e2b:C.3.3.4
fn write_c_3_3_4(
	xpath: &mut Context,
	base: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}//hl7:assignedPerson/hl7:name/hl7:given[2]"),
		value,
		null_flavor,
	);
}

/// e2b:C.3.3.5
fn write_c_3_3_5(
	xpath: &mut Context,
	base: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}//hl7:assignedPerson/hl7:name/hl7:family"),
		value,
		null_flavor,
	);
}

fn set_text_or_null_flavor(
	xpath: &mut Context,
	path: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
) {
	if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
		remove_attr_first(xpath, path, "nullFlavor");
		set_text_first(xpath, path, value);
	} else if let Some(null_flavor) =
		null_flavor.filter(|value| !value.trim().is_empty())
	{
		set_text_first(xpath, path, "");
		set_attr_first(xpath, path, "nullFlavor", null_flavor);
	}
}

/// e2b:C.3.4.1
fn write_c_3_4_1(xpath: &mut Context, base: &str, value: &str) {
	set_text_first(
		xpath,
		&format!("{base}/hl7:addr/hl7:streetAddressLine"),
		value,
	);
}

/// e2b:C.3.4.2
fn write_c_3_4_2(xpath: &mut Context, base: &str, value: &str) {
	set_text_first(xpath, &format!("{base}/hl7:addr/hl7:city"), value);
}

/// e2b:C.3.4.3
fn write_c_3_4_3(xpath: &mut Context, base: &str, value: &str) {
	set_text_first(xpath, &format!("{base}/hl7:addr/hl7:state"), value);
}

/// e2b:C.3.4.4
fn write_c_3_4_4(xpath: &mut Context, base: &str, value: &str) {
	set_text_first(xpath, &format!("{base}/hl7:addr/hl7:postalCode"), value);
}

/// e2b:C.3.4.5
fn write_c_3_4_5(xpath: &mut Context, base: &str, value: &str) {
	set_attr_first(
		xpath,
		&format!(
			"{base}//hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code"
		),
		"code",
		value,
	);
}

/// e2b:C.3.4.6
fn write_c_3_4_6(xpath: &mut Context, base: &str, value: Option<&str>) {
	write_c_3_4_telecom(xpath, base, "tel:", value);
}

/// e2b:C.3.4.7
fn write_c_3_4_7(xpath: &mut Context, base: &str, value: Option<&str>) {
	write_c_3_4_telecom(xpath, base, "fax:", value);
}

/// e2b:C.3.4.8
fn write_c_3_4_8(xpath: &mut Context, base: &str, value: Option<&str>) {
	write_c_3_4_telecom(xpath, base, "mailto:", value);
}

fn write_c_3_4_telecom(
	xpath: &mut Context,
	base: &str,
	prefix: &str,
	value: Option<&str>,
) {
	let path = format!("{base}/hl7:telecom[starts-with(@value,'{prefix}')]");
	let value = normalized_c_3_4_telecom(prefix, value);

	if let Some(value) = value {
		set_attr_first(xpath, &path, "value", &value);
	} else {
		remove_nodes(xpath, &path);
	}
}

fn normalized_c_3_4_telecom(prefix: &str, value: Option<&str>) -> Option<String> {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.and_then(|value| {
			let body = value.strip_prefix(prefix).unwrap_or(value).trim();
			(!body.is_empty()).then(|| format!("{prefix}{body}"))
		})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fda_c_1_7_1_uses_one_namespaced_value_type() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><PORR_IN049016UV><controlActProcess><subject><investigationEvent/></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let mut doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
			.unwrap();

		write_fda_c_1_7_1(&mut doc, &parser, &mut xpath, Some("1")).unwrap();

		let xml = doc.to_string();
		assert_eq!(xml.matches("xsi:type=\"CE\"").count(), 1, "{xml}");
		assert!(!xml.contains(" type=\"CE\""), "{xml}");
		assert!(
			xml.contains(
				"code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.1.1\""
			),
			"{xml}"
		);
	}

	#[test]
	fn c_1_7_value_clears_stale_null_flavor() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><component><observationEvent><code code="23" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value xsi:type="BL" nullFlavor="NI"/></observationEvent></component></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let mut doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
			.unwrap();

		write_c_1_7(&mut doc, &parser, &mut xpath, Some(true), None)
			.expect("patch C.1.7");
		let value = xpath
			.findnodes(
				"//hl7:observationEvent[hl7:code[@code='23']]/hl7:value",
				None,
			)
			.unwrap()
			.into_iter()
			.next()
			.unwrap();
		assert_eq!(value.get_attribute("value").as_deref(), Some("true"));
		assert!(value.get_attribute("nullFlavor").is_none());
		drop(value);

		write_c_1_7(&mut doc, &parser, &mut xpath, None, Some("NI"))
			.expect("patch C.1.7 nullFlavor");
		let value = xpath
			.findnodes(
				"//hl7:observationEvent[hl7:code[@code='23']]/hl7:value",
				None,
			)
			.unwrap()
			.into_iter()
			.next()
			.unwrap();
		assert!(value.get_attribute("value").is_none());
		assert_eq!(value.get_attribute("nullFlavor").as_deref(), Some("NI"));
	}

	#[test]
	fn c_1_8_2_inserts_relationship_when_export_skeleton_lacks_it() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent/></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let mut doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		write_c_1_8_2(&mut doc, &parser, &mut xpath, Some("1")).unwrap();

		assert_eq!(
			xpath
				.findvalue(
					"count(//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']])",
					None,
				)
				.unwrap(),
			"1"
		);
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']]//hl7:assignedEntity/hl7:code/@code",
					None,
				)
				.unwrap(),
			"1"
		);
	}

	#[test]
	fn c_1_8_2_relationship_precedes_existing_subject_nodes() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><id/><code/><text/><statusCode/><effectiveTime/><availabilityTime/><component/><subjectOf1/><subjectOf2/></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let mut doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		write_c_1_8_2(&mut doc, &parser, &mut xpath, Some("2")).unwrap();

		let event = xpath
			.findnodes("//hl7:investigationEvent", None)
			.unwrap()
			.into_iter()
			.next()
			.unwrap();
		let children = event
			.get_child_nodes()
			.into_iter()
			.filter(|node| {
				node.get_type() == Some(libxml::tree::NodeType::ElementNode)
			})
			.map(|node| node.get_name())
			.collect::<Vec<_>>();
		assert_eq!(
			children,
			[
				"id",
				"code",
				"text",
				"statusCode",
				"effectiveTime",
				"availabilityTime",
				"component",
				"outboundRelationship",
				"subjectOf1",
				"subjectOf2"
			]
		);
	}
}
