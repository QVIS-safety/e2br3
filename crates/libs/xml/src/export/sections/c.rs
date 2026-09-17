use super::*;
use crate::export::roundtrip::{patch_c_safety_report, CSafetyReportPatch};
use lib_core::model::case_identifiers::{LinkedReportNumber, OtherCaseIdentifier};
use lib_core::model::safety_report::{
	DocumentsHeldBySender, SafetyReportIdentification, SenderInformationBmc,
	SenderInformationFilter, StudyFdaCrossReportedInd, StudyFdaCrossReportedIndBmc,
	StudyFdaCrossReportedIndFilter, StudyInformationBmc, StudyInformationFilter,
	StudyRegistrationNumberBmc, StudyRegistrationNumberFilter,
};
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;

fn uuid_eq(id: sqlx::types::Uuid) -> OpValsValue {
	OpValValue::Eq(json!(id)).into()
}

pub(crate) async fn export_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
	raw_xml: &[u8],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<String> {
	let report = SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	let sender = fetch_sender_information(ctx, mm, case_id).await?;
	export_c_safety_report_patch(raw_xml, case, &report, sender.as_ref(), authority)
}

async fn fetch_sender_information(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<SenderInformation>> {
	let mut senders = SenderInformationBmc::list(
		ctx,
		mm,
		Some(vec![SenderInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	senders.sort_by_key(|sender| sender.created_at);
	Ok(senders.into_iter().next())
}

fn set_text_or_null_flavor(
	xpath: &mut Context,
	path: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
) {
	let value = value.map(str::trim).filter(|value| !value.is_empty());
	let null_flavor = null_flavor.map(str::trim).filter(|value| !value.is_empty());
	if let Ok(nodes) = xpath.findnodes(path, None) {
		for mut node in nodes.into_iter().take(1) {
			if let Some(value) = value {
				let _ = node.remove_attribute("nullFlavor");
				let _ = node.set_content(value);
			} else if let Some(null_flavor) = null_flavor {
				let _ = node.set_content("");
				let _ = node.set_attribute("nullFlavor", null_flavor);
			}
		}
	}
}

fn set_telecom_or_null_flavor(
	xpath: &mut Context,
	path: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
) {
	let value = value.map(str::trim).filter(|value| !value.is_empty());
	let null_flavor = null_flavor.map(str::trim).filter(|value| !value.is_empty());
	if let Ok(nodes) = xpath.findnodes(path, None) {
		for mut node in nodes.into_iter().take(1) {
			if let Some(value) = value {
				let telecom_value = if value.contains(':') {
					value.to_string()
				} else {
					format!("tel:{value}")
				};
				let _ = node.remove_attribute("nullFlavor");
				let _ = node.set_attribute("value", &telecom_value);
			} else if let Some(null_flavor) = null_flavor {
				let _ = node.set_attribute("value", "tel");
				let _ = node.set_attribute("nullFlavor", null_flavor);
			}
		}
	}
}

pub(crate) fn apply_c_2_primary_sources(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	sources: &[PrimarySource],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<()> {
	if sources.is_empty() {
		return Ok(());
	}
	remove_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]",
	);
	for (index, primary) in sources.iter().enumerate() {
		apply_c_2_primary_source_values(
			doc,
			parser,
			xpath,
			primary,
			authority,
			index + 1,
		)?;
	}
	crate::export::roundtrip::reorder_investigation_event_children(xpath);
	Ok(())
}

fn apply_c_2_primary_source_values(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	primary: &PrimarySource,
	authority: lib_core::regulatory::RegulatoryAuthority,
	index: usize,
) -> Result<()> {
	let relationship = format!("(//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']])[{index}]");
	let base = format!("{relationship}/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity");
	ensure_primary_source_author_nodes(
		doc,
		parser,
		xpath,
		primary,
		authority,
		&relationship,
		&base,
	)?;
	let has_organization = primary
		.organization
		.as_deref()
		.is_some_and(|v| !v.trim().is_empty())
		|| primary
			.organization_null_flavor
			.as_deref()
			.is_some_and(|v| !v.trim().is_empty());

	write_c_2_r_1_1(xpath, &base, primary);
	write_c_2_r_1_2(xpath, &base, primary);
	if primary
		.reporter_middle_name
		.as_deref()
		.is_some_and(|v| !v.trim().is_empty())
		|| primary
			.reporter_middle_name_null_flavor
			.as_deref()
			.is_some_and(|v| !v.trim().is_empty())
	{
		if xpath
			.findnodes(
				&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[2]"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&format!("{base}/hl7:assignedPerson/hl7:name"),
				"<given/>",
			)?;
		}
		write_c_2_r_1_3(xpath, &base, primary);
	}
	write_c_2_r_1_4(xpath, &base, primary);
	let has_department = primary
		.department
		.as_deref()
		.is_some_and(|v| !v.trim().is_empty())
		|| primary
			.department_null_flavor
			.as_deref()
			.is_some_and(|v| !v.trim().is_empty());
	if (has_organization || has_department)
		&& xpath
			.findnodes(&format!("{base}/hl7:representedOrganization"), None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
	{
		append_fragment_child(
			doc,
			parser,
			xpath,
			&base,
			"<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name/></representedOrganization>",
		)?;
	}
	let organization_path = if has_department {
		let nested = format!("{base}/hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name");
		if xpath
			.findnodes(&nested, None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&format!("{base}/hl7:representedOrganization"),
				"<assignedEntity classCode=\"ASSIGNED\"><representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name/></representedOrganization></assignedEntity>",
			)?;
		}
		nested
	} else {
		format!("{base}/hl7:representedOrganization/hl7:name")
	};
	write_c_2_r_2_1(xpath, &organization_path, primary);
	if has_department {
		write_c_2_r_2_2(xpath, &base, primary);
	}
	write_c_2_r_2_3(xpath, &base, primary);
	write_c_2_r_2_4(xpath, &base, primary);
	write_c_2_r_2_5(xpath, &base, primary);
	write_c_2_r_2_6(xpath, &base, primary);
	write_c_2_r_2_7(xpath, &base, primary);
	if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda) {
		write_fda_c_2_r_2_8(xpath, &base, primary);
	} else {
		remove_nodes(
			xpath,
			&format!("{base}/hl7:telecom[starts-with(@value,'mailto:')]"),
		);
	}
	write_c_2_r_3(xpath, &base, primary);
	if primary.qualification.is_some() || primary.qualification_null_flavor.is_some()
	{
		let path = format!("{base}/hl7:assignedPerson/hl7:asQualifiedEntity");
		if xpath
			.findnodes(&path, None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&format!("{base}/hl7:assignedPerson"),
				"<asQualifiedEntity><code/></asQualifiedEntity>",
			)?;
		}
	}
	write_c_2_r_4(xpath, &base, primary);
	if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Mfds) {
		write_c_2_r_4_kr_1(doc, parser, xpath, &base, primary)?;
	}
	write_c_2_r_5(xpath, &relationship, primary);

	Ok(())
}

/// e2b:C.2.r.1.1
fn write_c_2_r_1_1(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:assignedPerson/hl7:name/hl7:prefix"),
		value.reporter_title.as_deref(),
		value.reporter_title_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.1.2
fn write_c_2_r_1_2(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[1]"),
		value.reporter_given_name.as_deref(),
		value.reporter_given_name_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.1.3
fn write_c_2_r_1_3(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[2]"),
		value.reporter_middle_name.as_deref(),
		value.reporter_middle_name_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.1.4
fn write_c_2_r_1_4(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:assignedPerson/hl7:name/hl7:family"),
		value.reporter_family_name.as_deref(),
		value.reporter_family_name_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.2.1
fn write_c_2_r_2_1(xpath: &mut Context, path: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		path,
		value.organization.as_deref(),
		value.organization_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.2.2
fn write_c_2_r_2_2(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:representedOrganization/hl7:name"),
		value.department.as_deref(),
		value.department_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.2.3
fn write_c_2_r_2_3(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:addr/hl7:streetAddressLine"),
		value.street.as_deref(),
		value.street_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.2.4
fn write_c_2_r_2_4(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:addr/hl7:city"),
		value.city.as_deref(),
		value.city_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.2.5
fn write_c_2_r_2_5(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:addr/hl7:state"),
		value.state.as_deref(),
		value.state_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.2.6
fn write_c_2_r_2_6(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_text_or_null_flavor(
		xpath,
		&format!("{base}/hl7:addr/hl7:postalCode"),
		value.postcode.as_deref(),
		value.postcode_null_flavor.as_deref(),
	);
}

/// e2b:C.2.r.2.7
fn write_c_2_r_2_7(xpath: &mut Context, base: &str, value: &PrimarySource) {
	set_telecom_or_null_flavor(
		xpath,
		&format!("{base}/hl7:telecom[starts-with(@value,'tel:') or @nullFlavor]"),
		value.telephone.as_deref(),
		value.telephone_null_flavor.as_deref(),
	);
}

/// e2b:FDA.C.2.r.2.8
fn write_fda_c_2_r_2_8(xpath: &mut Context, base: &str, value: &PrimarySource) {
	let path = format!("{base}/hl7:telecom[starts-with(@value,'mailto')]");
	if let Some(email) = value.email.as_deref() {
		let email = if email.contains(':') {
			email.to_string()
		} else {
			format!("mailto:{email}")
		};
		set_attr_first(xpath, &path, "value", &email);
		remove_attr_first(xpath, &path, "nullFlavor");
	} else if let Some(null_flavor) = value.email_null_flavor.as_deref() {
		set_attr_first(xpath, &path, "value", "mailto");
		set_attr_first(xpath, &path, "nullFlavor", null_flavor);
	}
}

/// e2b:C.2.r.3
fn write_c_2_r_3(xpath: &mut Context, base: &str, value: &PrimarySource) {
	let path = format!(
		"{base}/hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code"
	);
	if let Some(code) = value.country_code.as_deref() {
		set_attr_first(xpath, &path, "code", code);
	} else if let Ok(nodes) = xpath.findnodes(&path, None) {
		for mut node in nodes.into_iter().take(1) {
			node.unlink_node();
		}
	}
}

/// e2b:C.2.r.4
fn write_c_2_r_4(xpath: &mut Context, base: &str, value: &PrimarySource) {
	let path = format!("{base}/hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code");
	match (
		value.qualification.as_deref(),
		value.qualification_null_flavor.as_deref(),
	) {
		(Some(code), _) => {
			set_attr_first(xpath, &path, "code", code);
			remove_attr_first(xpath, &path, "nullFlavor");
		}
		(None, Some(null_flavor)) => {
			set_attr_first(xpath, &path, "nullFlavor", null_flavor);
			remove_attr_first(xpath, &path, "code");
		}
		(None, None) => {}
	}
}

/// e2b:C.2.r.4.KR.1
fn write_c_2_r_4_kr_1(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	base: &str,
	value: &PrimarySource,
) -> Result<()> {
	const CODE_SYSTEM: &str = "2.16.840.1.113883.3.989.5.1.10.1.1";
	let path = format!(
		"{base}/hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code[@codeSystem='{CODE_SYSTEM}']"
	);
	if let Some(code) = value.qualification_kr1.as_deref() {
		let nodes = xpath
			.findnodes(&path, None)
			.map_err(|_| Error::InvalidXml {
				message: format!("Failed to find nodes for path {path}"),
				line: None,
				column: None,
			})?;
		if nodes.is_empty() {
			append_fragment_child(
				doc,
				parser,
				xpath,
				&format!("{base}/hl7:assignedPerson"),
				&format!("<asQualifiedEntity><code codeSystem=\"{CODE_SYSTEM}\" code=\"{}\"/></asQualifiedEntity>", xml_escape(code)),
			)?;
		} else {
			set_attr_first(xpath, &path, "code", code);
		}
	} else {
		remove_nodes(xpath, &format!(
			"{base}/hl7:assignedPerson/hl7:asQualifiedEntity[hl7:code[@codeSystem='{CODE_SYSTEM}']]"
		));
	}
	Ok(())
}

/// e2b:C.2.r.5
fn write_c_2_r_5(xpath: &mut Context, relationship: &str, value: &PrimarySource) {
	let path = format!("{relationship}/hl7:priorityNumber");
	if let Some(priority) = value.primary_source_regulatory.as_deref() {
		set_attr_first(xpath, &path, "value", priority);
	} else {
		remove_nodes(xpath, &path);
	}
}

fn ensure_primary_source_author_nodes(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	primary: &PrimarySource,
	authority: lib_core::regulatory::RegulatoryAuthority,
	relationship: &str,
	base: &str,
) -> Result<()> {
	if xpath
		.findnodes(base, None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let present = |value: Option<&str>, null_flavor: Option<&str>| {
		[value, null_flavor]
			.into_iter()
			.flatten()
			.any(|value| !value.trim().is_empty())
	};
	let title = present(
		primary.reporter_title.as_deref(),
		primary.reporter_title_null_flavor.as_deref(),
	)
	.then_some("<prefix/>")
	.unwrap_or_default();
	let given_name = present(
		primary.reporter_given_name.as_deref(),
		primary.reporter_given_name_null_flavor.as_deref(),
	);
	let middle_name = present(
		primary.reporter_middle_name.as_deref(),
		primary.reporter_middle_name_null_flavor.as_deref(),
	);
	let given = given_name.then_some("<given/>").unwrap_or_default();
	let middle = (middle_name && given_name)
		.then_some("<given/>")
		.unwrap_or_default();
	let family = present(
		primary.reporter_family_name.as_deref(),
		primary.reporter_family_name_null_flavor.as_deref(),
	)
	.then_some("<family/>")
	.unwrap_or_default();
	let name = (!title.is_empty()
		|| !given.is_empty()
		|| !middle.is_empty()
		|| !family.is_empty())
	.then(|| format!("<name>{title}{given}{middle}{family}</name>"))
	.unwrap_or_default();
	let qualification = present(
		primary.qualification.as_deref(),
		primary.qualification_null_flavor.as_deref(),
	)
	.then_some("<asQualifiedEntity classCode=\"QUAL\"><code codeSystem=\"2.16.840.1.113883.3.989.2.1.1.6\"/></asQualifiedEntity>")
	.unwrap_or_default();
	let location = primary
		.country_code
		.as_deref()
		.filter(|value| !value.trim().is_empty())
		.map(|_| "<asLocatedEntity classCode=\"LOCE\"><location classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code codeSystem=\"1.0.3166.1.2.2\"/></location></asLocatedEntity>")
		.unwrap_or_default();
	let assigned_person = (!name.is_empty()
		|| !qualification.is_empty()
		|| !location.is_empty())
		.then(|| format!("<assignedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\">{name}{qualification}{location}</assignedPerson>"))
		.unwrap_or_default();
	let street = present(
		primary.street.as_deref(),
		primary.street_null_flavor.as_deref(),
	)
	.then_some("<streetAddressLine/>")
	.unwrap_or_default();
	let city = present(primary.city.as_deref(), primary.city_null_flavor.as_deref())
		.then_some("<city/>")
		.unwrap_or_default();
	let state = present(
		primary.state.as_deref(),
		primary.state_null_flavor.as_deref(),
	)
	.then_some("<state/>")
	.unwrap_or_default();
	let postcode = present(
		primary.postcode.as_deref(),
		primary.postcode_null_flavor.as_deref(),
	)
	.then_some("<postalCode/>")
	.unwrap_or_default();
	let address = (!street.is_empty()
		|| !city.is_empty()
		|| !state.is_empty()
		|| !postcode.is_empty())
	.then(|| format!("<addr>{street}{city}{state}{postcode}</addr>"))
	.unwrap_or_default();
	let telephone = present(
		primary.telephone.as_deref(),
		primary.telephone_null_flavor.as_deref(),
	)
	.then_some("<telecom value=\"tel:\"/>")
	.unwrap_or_default();
	let email =
		(matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda)
			&& present(
				primary.email.as_deref(),
				primary.email_null_flavor.as_deref(),
			))
		.then_some("<telecom value=\"mailto:\"/>")
		.unwrap_or_default();
	let has_organization = present(
		primary.organization.as_deref(),
		primary.organization_null_flavor.as_deref(),
	);
	let has_department = present(
		primary.department.as_deref(),
		primary.department_null_flavor.as_deref(),
	);
	let organization = if has_department {
		let nested = has_organization
			.then_some("<assignedEntity classCode=\"ASSIGNED\"><representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name/></representedOrganization></assignedEntity>")
			.unwrap_or_default();
		format!("<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name/>{nested}</representedOrganization>")
	} else if has_organization {
		"<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name/></representedOrganization>".to_string()
	} else {
		String::new()
	};
	let author = format!(
		"<subjectOf2 typeCode=\"SUBJ\"><controlActEvent classCode=\"CACT\" moodCode=\"EVN\"><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\">{address}{telephone}{email}{assigned_person}{organization}</assignedEntity></author></controlActEvent></subjectOf2>"
	);

	if xpath
		.findnodes(relationship, None)
		.map(|nodes| nodes.is_empty())
		.unwrap_or(true)
	{
		let priority = primary
			.primary_source_regulatory
			.as_deref()
			.filter(|value| !value.trim().is_empty())
			.map(|_| "<priorityNumber/>")
			.unwrap_or_default();
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent",
			&format!(
				"<outboundRelationship typeCode=\"SPRT\">{priority}<relatedInvestigation classCode=\"INVSTG\" moodCode=\"EVN\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.22\"/>{author}</relatedInvestigation></outboundRelationship>"
			),
		)?;
		return Ok(());
	}

	append_fragment_child(
		doc,
		parser,
		xpath,
		&format!("{relationship}/hl7:relatedInvestigation"),
		&author,
	)
}

pub(crate) fn apply_c_1_report_relationships(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	documents: &[DocumentsHeldBySender],
	identifiers: &[OtherCaseIdentifier],
	linked_reports: &[LinkedReportNumber],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<()> {
	remove_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:reference[hl7:document/hl7:code[@code='1']]",
	);
	remove_nodes(xpath, "//hl7:investigationEvent/hl7:subjectOf1[hl7:controlActEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.3']]");
	remove_nodes(xpath, "//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code/@nullFlavor='NA']");

	let mut fragment = String::new();
	for value in identifiers {
		fragment.push_str(&write_c_1_9_1(value));
	}
	for value in linked_reports {
		fragment.push_str(&write_c_1_10_r(value));
	}
	for value in documents {
		fragment.push_str(&format!("<reference typeCode=\"REFR\"><document classCode=\"DOC\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.27\"/>{}{}</document></reference>", write_c_1_6_1_r_1(value), write_c_1_6_1_r_2(value, authority)?));
	}
	if fragment.is_empty() {
		return Ok(());
	}
	let Some(xml) = insert_c_1_or_c_4(&doc.to_string(), &fragment) else {
		return Ok(());
	};
	*doc = parser.parse_string(&xml).map_err(|err| Error::InvalidXml {
		message: format!("XML parse error after C relationship injection: {err}"),
		line: None,
		column: None,
	})?;
	*xpath = Context::new(doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath after C relationship injection"
			.to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let _ =
		xpath.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
	crate::export::roundtrip::reorder_investigation_event_children(xpath);
	Ok(())
}

/// e2b:C.1.6.1.r.1
fn write_c_1_6_1_r_1(value: &DocumentsHeldBySender) -> String {
	value
		.title
		.as_deref()
		.filter(|v| !v.trim().is_empty())
		.map(|v| format!("<title>{}</title>", xml_escape(v)))
		.unwrap_or_default()
}

/// e2b:C.1.6.1.r.2
fn write_c_1_6_1_r_2(
	value: &DocumentsHeldBySender,
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<String> {
	write_attachment_text(
		value.document_base64.as_deref(),
		value.file_name.as_deref(),
		value.media_type.as_deref(),
		value.representation.as_deref(),
		value.compression.as_deref(),
		authority,
		"C.1.6.1.r.2",
	)
}

fn write_attachment_text(
	document: Option<&str>,
	file_name: Option<&str>,
	media_type: Option<&str>,
	representation: Option<&str>,
	compression: Option<&str>,
	authority: lib_core::regulatory::RegulatoryAuthority,
	_field_code: &str,
) -> Result<String> {
	let Some(document) = document.filter(|value| !value.trim().is_empty()) else {
		return Ok(String::new());
	};
	let file_name = file_name.map(str::trim).filter(|value| !value.is_empty());
	let mut media_type = media_type
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.unwrap_or("application/octet-stream");
	if authority == lib_core::regulatory::RegulatoryAuthority::Fda {
		media_type = file_name
			.and_then(lib_core::regulatory::fda_attachment_media_type)
			.unwrap_or(media_type);
	}
	let reference = file_name
		.map(|value| format!("<reference value=\"{}\"/>", xml_escape(value)))
		.unwrap_or_default();
	let representation = representation
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.unwrap_or("B64");
	let compression = compression
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| format!(" compression=\"{}\"", xml_escape(value)))
		.unwrap_or_default();
	Ok(format!(
		"<text mediaType=\"{}\" representation=\"{}\"{}>{}{}</text>",
		xml_escape(media_type),
		xml_escape(representation),
		compression,
		reference,
		xml_escape(document)
	))
}

fn write_c_1_9_1(value: &OtherCaseIdentifier) -> String {
	format!("<subjectOf1 typeCode=\"SUBJ\"><controlActEvent classCode=\"CACT\" moodCode=\"EVN\"><id root=\"2.16.840.1.113883.3.989.2.1.3.3\" assigningAuthorityName=\"{}\" extension=\"{}\"/></controlActEvent></subjectOf1>", write_c_1_9_1_r_1(value), write_c_1_9_1_r_2(value))
}

/// e2b:C.1.9.1.r.1
fn write_c_1_9_1_r_1(value: &OtherCaseIdentifier) -> String {
	xml_escape(&value.source_of_identifier)
}

/// e2b:C.1.9.1.r.2
fn write_c_1_9_1_r_2(value: &OtherCaseIdentifier) -> String {
	xml_escape(&value.case_identifier)
}

/// e2b:C.1.10.r
fn write_c_1_10_r(value: &LinkedReportNumber) -> String {
	format!("<outboundRelationship typeCode=\"SPRT\"><relatedInvestigation classCode=\"INVSTG\" moodCode=\"EVN\"><code nullFlavor=\"NA\"/><subjectOf2 typeCode=\"SUBJ\"><controlActEvent classCode=\"CACT\" moodCode=\"EVN\"><id root=\"2.16.840.1.113883.3.989.2.1.3.2\" extension=\"{}\"/></controlActEvent></subjectOf2></relatedInvestigation></outboundRelationship>", xml_escape(&value.linked_report_number))
}

pub fn export_c_safety_report_patch(
	raw_xml: &[u8],
	_case: &Case,
	report: &SafetyReportIdentification,
	sender: Option<&SenderInformation>,
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<String> {
	let patch = CSafetyReportPatch {
		report_unique_id: report.safety_report_id.as_deref().unwrap_or(""),
		transmission_date: report.transmission_date.as_deref(),
		report_type: report.report_type.as_deref().unwrap_or(""),
		date_first_received: report.date_first_received_from_source.as_deref(),
		date_most_recent: report.date_of_most_recent_information.as_deref(),
		fulfil_expedited: report.fulfil_expedited_criteria,
		fulfil_expedited_null_flavor: report
			.fulfil_expedited_criteria_null_flavor
			.as_deref(),
		additional_documents_available: report.additional_documents_available,
		other_case_identifiers_exist: report.other_case_identifiers_exist,
		other_case_identifiers_exist_null_flavor: report
			.other_case_identifiers_exist_null_flavor
			.as_deref(),
		worldwide_unique_id: report.worldwide_unique_id.as_deref(),
		first_sender_type: report.first_sender_type.as_deref(),
		local_criteria_report_type: matches!(
			authority,
			lib_core::regulatory::RegulatoryAuthority::Fda
		)
		.then_some(report.local_criteria_report_type.as_deref())
		.flatten(),
		combination_product_indicator: matches!(
			authority,
			lib_core::regulatory::RegulatoryAuthority::Fda
		)
		.then_some(report.combination_product_report_indicator.as_deref())
		.flatten(),
		combination_product_indicator_null_flavor: matches!(
			authority,
			lib_core::regulatory::RegulatoryAuthority::Fda
		)
		.then_some(
			report
				.combination_product_report_indicator_null_flavor
				.as_deref(),
		)
		.flatten(),
		nullification_code: report.nullification_code.as_deref(),
		nullification_reason: report.nullification_reason.as_deref(),
		sender_type: sender.and_then(|s| s.sender_type.as_deref()),
		sender_health_professional_type_kr1: matches!(
			authority,
			lib_core::regulatory::RegulatoryAuthority::Mfds
		)
		.then(|| sender.and_then(|s| s.health_professional_type_kr1.as_deref()))
		.flatten(),
		sender_org_name: sender.and_then(|s| s.organization_name.as_deref()),
		sender_department: sender.and_then(|s| s.department.as_deref()),
		sender_street_address: sender.and_then(|s| s.street_address.as_deref()),
		sender_city: sender.and_then(|s| s.city.as_deref()),
		sender_state: sender.and_then(|s| s.state.as_deref()),
		sender_postcode: sender.and_then(|s| s.postcode.as_deref()),
		sender_country_code: sender.and_then(|s| s.country_code.as_deref()),
		sender_person_title: sender.and_then(|s| s.person_title.as_deref()),
		sender_person_title_null_flavor: sender
			.and_then(|s| s.person_title_null_flavor.as_deref()),
		sender_person_given_name: sender
			.and_then(|s| s.person_given_name.as_deref()),
		sender_person_given_name_null_flavor: sender
			.and_then(|s| s.person_given_name_null_flavor.as_deref()),
		sender_person_middle_name: sender
			.and_then(|s| s.person_middle_name.as_deref()),
		sender_person_middle_name_null_flavor: sender
			.and_then(|s| s.person_middle_name_null_flavor.as_deref()),
		sender_person_family_name: sender
			.and_then(|s| s.person_family_name.as_deref()),
		sender_person_family_name_null_flavor: sender
			.and_then(|s| s.person_family_name_null_flavor.as_deref()),
		sender_telephone: sender.and_then(|s| s.telephone.as_deref()),
		sender_fax: sender.and_then(|s| s.fax.as_deref()),
		sender_email: sender.and_then(|s| s.email.as_deref()),
	};

	patch_c_safety_report(raw_xml, &patch)
}

#[cfg(test)]
mod primary_source_null_flavor_tests {
	use super::*;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;
	use std::collections::BTreeSet;

	fn source() -> PrimarySource {
		PrimarySource {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			source_reporter_presave_id: None,
			sequence_number: 1,
			reporter_title: None,
			reporter_title_null_flavor: None,
			reporter_given_name: None,
			reporter_given_name_null_flavor: Some("ASKU".to_string()),
			reporter_middle_name: None,
			reporter_middle_name_null_flavor: None,
			reporter_family_name: None,
			reporter_family_name_null_flavor: None,
			organization: None,
			organization_null_flavor: None,
			department: None,
			department_null_flavor: None,
			street: None,
			street_null_flavor: None,
			city: None,
			city_null_flavor: Some("NASK".to_string()),
			state: None,
			state_null_flavor: None,
			postcode: None,
			postcode_null_flavor: None,
			telephone: None,
			telephone_null_flavor: None,
			country_code: None,
			email: None,
			email_null_flavor: None,
			qualification: None,
			qualification_null_flavor: None,
			qualification_kr1: None,
			primary_source_regulatory: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	#[test]
	fn primary_source_export_creates_required_relationship_from_fresh_skeleton() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(crate::export::base_export_skeleton())
			.expect("parse skeleton");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		let mut primary = source();
		primary.reporter_middle_name = Some("Q".to_string());
		primary.street = Some("13 Elm St".to_string());
		primary.city = Some("Metropolis".to_string());
		primary.city_null_flavor = None;
		primary.telephone = Some("6102227777".to_string());
		primary.country_code = Some("US".to_string());
		primary.email = Some("reporter@example.com".to_string());
		primary.qualification = Some("1".to_string());
		primary.primary_source_regulatory = Some("1".to_string());

		apply_c_2_primary_source_values(
			&mut doc,
			&parser,
			&mut xpath,
			&primary,
			lib_core::regulatory::RegulatoryAuthority::Fda,
			1,
		)
		.expect("apply primary source");

		let relationship = "//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]";
		assert_eq!(xpath.findnodes(relationship, None).unwrap().len(), 1);
		assert_eq!(
			xpath
				.findvalue(
					&format!("{relationship}/hl7:priorityNumber/@value"),
					None
				)
				.unwrap(),
			"1"
		);
		assert_eq!(
			xpath
				.findvalue(
					&format!("{relationship}//hl7:addr/hl7:streetAddressLine"),
					None
				)
				.unwrap(),
			"13 Elm St"
		);
		assert_eq!(
			xpath
				.findvalue(
					&format!("{relationship}//hl7:asQualifiedEntity/hl7:code/@code"),
					None
				)
				.unwrap(),
			"1"
		);

		let node = xpath
			.findnodes(relationship, None)
			.unwrap()
			.into_iter()
			.next()
			.unwrap();
		let children = node
			.get_child_nodes()
			.into_iter()
			.filter(|node| {
				node.get_type() == Some(libxml::tree::NodeType::ElementNode)
			})
			.map(|node| node.get_name())
			.collect::<Vec<_>>();
		assert_eq!(children, ["priorityNumber", "relatedInvestigation"]);
	}

	#[test]
	fn mfds_primary_source_keeps_common_and_kr_qualification_codes() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(crate::export::base_export_skeleton())
			.expect("parse skeleton");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		let mut primary = source();
		primary.qualification = Some("3".to_string());
		primary.qualification_kr1 = Some("1".to_string());

		apply_c_2_primary_source_values(
			&mut doc,
			&parser,
			&mut xpath,
			&primary,
			lib_core::regulatory::RegulatoryAuthority::Mfds,
			1,
		)
		.expect("apply MFDS primary source");

		let common = "//hl7:asQualifiedEntity/hl7:code[@codeSystem='2.16.840.1.113883.3.989.2.1.1.6']/@code";
		let kr = "//hl7:asQualifiedEntity/hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.10.1.1']/@code";
		assert_eq!(xpath.findvalue(common, None).unwrap(), "3");
		assert_eq!(xpath.findvalue(kr, None).unwrap(), "1");
	}

	#[test]
	fn sparse_primary_source_does_not_create_empty_optional_nodes() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(crate::export::base_export_skeleton())
			.expect("parse skeleton");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		let mut primary = source();
		primary.reporter_given_name_null_flavor = None;
		primary.city_null_flavor = None;
		primary.qualification = Some("1".to_string());
		primary.primary_source_regulatory = Some("1".to_string());

		apply_c_2_primary_source_values(
			&mut doc,
			&parser,
			&mut xpath,
			&primary,
			lib_core::regulatory::RegulatoryAuthority::Fda,
			1,
		)
		.expect("apply sparse primary source");

		let relationship = "//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]";
		for path in [
			"//hl7:assignedPerson/hl7:name",
			"//hl7:assignedEntity/hl7:addr",
			"//hl7:assignedEntity/hl7:telecom",
			"//hl7:assignedEntity/hl7:representedOrganization",
		] {
			assert!(
				xpath
					.findnodes(&format!("{relationship}{path}"), None)
					.unwrap()
					.is_empty(),
				"unexpected optional node: {path}"
			);
		}
		assert_eq!(
			xpath
				.findvalue(
					&format!("{relationship}//hl7:asQualifiedEntity/hl7:code/@code"),
					None,
				)
				.unwrap(),
			"1"
		);
	}

	#[test]
	fn primary_source_export_isolates_element_null_flavors() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><outboundRelationship><relatedInvestigation><code code="2"/><subjectOf2><controlActEvent><author><assignedEntity><assignedPerson><name><prefix/><given/><family/></name></assignedPerson><representedOrganization><name/></representedOrganization><addr><streetAddressLine/><city/><state/><postalCode/></addr><telecom value="tel:"/></assignedEntity></author></controlActEvent></subjectOf2></relatedInvestigation></outboundRelationship></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let mut doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		apply_c_2_primary_source_values(
			&mut doc,
			&parser,
			&mut xpath,
			&source(),
			lib_core::regulatory::RegulatoryAuthority::Fda,
			1,
		)
		.expect("apply primary source");

		assert_eq!(
			xpath
				.findvalue(
					"//hl7:assignedPerson/hl7:name/hl7:given[1]/@nullFlavor",
					None
				)
				.unwrap(),
			"ASKU"
		);
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:assignedEntity/hl7:addr/hl7:city/@nullFlavor",
					None
				)
				.unwrap(),
			"NASK"
		);
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:assignedPerson/hl7:name/hl7:prefix/@nullFlavor",
					None
				)
				.unwrap(),
			""
		);
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:assignedEntity/hl7:addr/hl7:state/@nullFlavor",
					None
				)
				.unwrap(),
			""
		);
	}

	#[test]
	fn primary_source_export_switches_qualification_value_and_null_flavor() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><outboundRelationship><relatedInvestigation><code code="2"/><subjectOf2><controlActEvent><author><assignedEntity><assignedPerson><name/></assignedPerson><representedOrganization><name/></representedOrganization><addr/></assignedEntity></author></controlActEvent></subjectOf2></relatedInvestigation></outboundRelationship></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let mut doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		let mut primary = source();
		primary.qualification_null_flavor = Some("UNK".to_string());

		apply_c_2_primary_source_values(
			&mut doc,
			&parser,
			&mut xpath,
			&primary,
			lib_core::regulatory::RegulatoryAuthority::Ich,
			1,
		)
		.expect("apply null flavor");
		let path = "//hl7:asQualifiedEntity/hl7:code";
		assert_eq!(
			xpath
				.findvalue(&format!("{path}/@nullFlavor"), None)
				.unwrap(),
			"UNK"
		);
		assert_eq!(xpath.findvalue(&format!("{path}/@code"), None).unwrap(), "");

		primary.qualification = Some("3".to_string());
		primary.qualification_null_flavor = None;
		apply_c_2_primary_source_values(
			&mut doc,
			&parser,
			&mut xpath,
			&primary,
			lib_core::regulatory::RegulatoryAuthority::Ich,
			1,
		)
		.expect("apply value");
		assert_eq!(
			xpath.findvalue(&format!("{path}/@code"), None).unwrap(),
			"3"
		);
		assert_eq!(
			xpath
				.findvalue(&format!("{path}/@nullFlavor"), None)
				.unwrap(),
			""
		);
	}

	#[test]
	fn primary_source_export_keeps_repeated_reporters_separate() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(crate::export::base_export_skeleton())
			.expect("parse skeleton");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		let mut first = source();
		first.reporter_given_name = Some("First".to_string());
		first.reporter_given_name_null_flavor = None;
		first.qualification = Some("5".to_string());
		let mut second = source();
		second.reporter_given_name = Some("Second".to_string());
		second.reporter_given_name_null_flavor = None;
		second.qualification = Some("1".to_string());
		second.primary_source_regulatory = Some("1".to_string());

		for (index, primary) in [&first, &second].into_iter().enumerate() {
			apply_c_2_primary_source_values(
				&mut doc,
				&parser,
				&mut xpath,
				primary,
				lib_core::regulatory::RegulatoryAuthority::Ich,
				index + 1,
			)
			.expect("apply reporter");
		}

		let relationship = "//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]";
		assert_eq!(xpath.findnodes(relationship, None).unwrap().len(), 2);
		for (index, name, qualification, priority) in
			[(1, "First", "5", ""), (2, "Second", "1", "1")]
		{
			let row = format!("({relationship})[{index}]");
			assert_eq!(
				xpath
					.findvalue(&format!("{row}//hl7:name/hl7:given[1]"), None)
					.unwrap(),
				name
			);
			assert_eq!(
				xpath
					.findvalue(
						&format!("{row}//hl7:asQualifiedEntity/hl7:code/@code"),
						None
					)
					.unwrap(),
				qualification
			);
			assert_eq!(
				xpath
					.findvalue(&format!("{row}/hl7:priorityNumber/@value"), None)
					.unwrap(),
				priority
			);
		}
	}

	#[test]
	fn primary_source_email_is_fda_only() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><outboundRelationship><relatedInvestigation><code code="2"/><subjectOf2><controlActEvent><author><assignedEntity><assignedPerson><name/></assignedPerson><representedOrganization><name/></representedOrganization><addr/><telecom value="mailto:old@example.com"/></assignedEntity></author></controlActEvent></subjectOf2></relatedInvestigation></outboundRelationship></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let mut doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		let mut primary = source();
		primary.email = Some("new@example.com".to_string());

		apply_c_2_primary_source_values(
			&mut doc,
			&parser,
			&mut xpath,
			&primary,
			lib_core::regulatory::RegulatoryAuthority::Mfds,
			1,
		)
		.expect("apply primary source");

		assert!(!doc.to_string().contains("mailto:"));
	}

	#[test]
	fn fda_telecom_null_flavors_keep_required_discriminators() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><outboundRelationship><relatedInvestigation><code code="2"/><subjectOf2><controlActEvent><author><assignedEntity><assignedPerson><name/></assignedPerson><representedOrganization><name/></representedOrganization><addr/><telecom value="tel:"/><telecom value="mailto:"/></assignedEntity></author></controlActEvent></subjectOf2></relatedInvestigation></outboundRelationship></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let mut doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		let mut primary = source();
		primary.telephone = None;
		primary.telephone_null_flavor = Some("NASK".to_string());
		primary.email = None;
		primary.email_null_flavor = Some("ASKU".to_string());

		apply_c_2_primary_source_values(
			&mut doc,
			&parser,
			&mut xpath,
			&primary,
			lib_core::regulatory::RegulatoryAuthority::Fda,
			1,
		)
		.expect("apply primary source");

		let output = doc.to_string();
		assert!(
			output.contains("value=\"tel\" nullFlavor=\"NASK\"")
				|| output.contains("nullFlavor=\"NASK\" value=\"tel\"")
		);
		assert!(
			output.contains("value=\"mailto\" nullFlavor=\"ASKU\"")
				|| output.contains("nullFlavor=\"ASKU\" value=\"mailto\"")
		);
	}

	#[test]
	fn fda_attachment_keeps_file_name_and_infers_media_type() {
		let authority = lib_core::regulatory::RegulatoryAuthority::Fda;
		let xml = write_attachment_text(
			Some("QUJD"),
			Some("report.pdf"),
			Some("application/pdf"),
			Some("B64"),
			None,
			authority,
			"C.4.r.2",
		)
		.expect("valid FDA attachment");
		assert!(xml.contains("<reference value=\"report.pdf\"/>QUJD"));
		let corrected = write_attachment_text(
			Some("QUJD"),
			Some("report.pdf"),
			Some("text/plain"),
			Some("B64"),
			None,
			authority,
			"C.4.r.2",
		)
		.expect("semantic media mismatch must not block export");
		assert!(corrected.contains("mediaType=\"application/pdf\""));
	}

	#[test]
	fn section_c_writers_cover_registry_fields() {
		let registry: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../../../registry/sections/c-safety-report.json"
		))
		.expect("section C registry");
		let expected = registry
			.as_array()
			.expect("registry array")
			.iter()
			.filter(|entry| entry["local_only"] != true)
			.filter_map(|entry| entry["e2br3_code"].as_str())
			.collect::<BTreeSet<_>>();
		let source = format!(
			"{}\n{}",
			include_str!("c.rs"),
			include_str!("../roundtrip/c_safety_report.rs")
		);
		let implemented = source
			.lines()
			.filter_map(|line| line.trim().strip_prefix("/// e2b:"))
			.collect::<BTreeSet<_>>();

		assert_eq!(implemented, expected);
	}
}

pub(crate) fn apply_c_4_literature(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	references: &[LiteratureReference],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<()> {
	if references.is_empty() {
		return Ok(());
	}

	remove_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:reference[hl7:document/hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.27']]",
	);

	let mut fragment = String::new();
	for item in references {
		let bibliographic = write_c_4_r_1(item);
		let attachment = write_c_4_r_2(item, authority)?;
		fragment.push_str(&format!(
			"<reference typeCode=\"REFR\"><document classCode=\"DOC\" moodCode=\"EVN\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.27\"/>{attachment}{bibliographic}</document></reference>"
		));
	}

	let xml = doc.to_string();
	if let Some(injected) = insert_c_1_or_c_4(&xml, &fragment) {
		let new_doc =
			parser
				.parse_string(&injected)
				.map_err(|err| Error::InvalidXml {
					message: format!(
						"XML parse error after literature injection: {err}"
					),
					line: None,
					column: None,
				})?;
		*doc = new_doc;
		*xpath = Context::new(doc).map_err(|_| Error::InvalidXml {
			message: "Failed to initialize XPath context after literature injection"
				.to_string(),
			line: None,
			column: None,
		})?;
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		let _ = xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
	}
	Ok(())
}

/// e2b:C.4.r.1
fn write_c_4_r_1(item: &LiteratureReference) -> String {
	let text = item.reference_text.as_deref().unwrap_or("").trim();
	if !text.is_empty() {
		return format!(
			"<bibliographicDesignationText>{}</bibliographicDesignationText>",
			xml_escape(text)
		);
	}
	item.reference_text_null_flavor
		.as_deref()
		.map(|value| {
			format!(
				"<bibliographicDesignationText nullFlavor=\"{}\"/>",
				xml_escape(value)
			)
		})
		.unwrap_or_else(|| "<bibliographicDesignationText/>".to_string())
}

/// e2b:C.4.r.2
fn write_c_4_r_2(
	item: &LiteratureReference,
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<String> {
	write_attachment_text(
		item.document_base64.as_deref(),
		item.file_name.as_deref(),
		item.media_type.as_deref(),
		item.representation.as_deref(),
		item.compression.as_deref(),
		authority,
		"C.4.r.2",
	)
}

pub(crate) fn apply_c_5_study(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	study: Option<&StudyInformation>,
	registrations: &[StudyRegistrationNumber],
	cross_reported_inds: &[StudyFdaCrossReportedInd],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<()> {
	let Some(study) = study else {
		return Ok(());
	};

	remove_nodes(xpath, "//hl7:primaryRole/hl7:subjectOf1[hl7:researchStudy]");
	remove_nodes(xpath, "//hl7:primaryRole/hl7:subjectOf2[hl7:researchStudy]");
	remove_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:subjectOf2[hl7:investigationCharacteristic/hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.5.1.10.1.7']]",
	);

	let mut auth_xml = String::new();
	for reg in registrations {
		if reg
			.registration_number
			.as_deref()
			.is_none_or(|value| value.trim().is_empty())
			&& reg.registration_number_null_flavor.is_none()
		{
			continue;
		}
		let country_xml = write_c_5_1_r_2(reg);
		let id_xml = write_c_5_1_r_1(reg);
		auth_xml.push_str(&format!(
			"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\">{id_xml}{country_xml}</studyRegistration></authorization>"
		));
	}

	if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda) {
		let mut fda_ids = vec![write_fda_c_5_5a(study), write_fda_c_5_5b(study)];
		fda_ids.extend(cross_reported_inds.iter().map(write_fda_c_5_6_r));
		for id in fda_ids.into_iter().filter(|id| !id.is_empty()) {
			auth_xml.push_str(&format!("<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\">{id}</studyRegistration></authorization>"));
		}
	}

	let sponsor_id_xml = write_c_5_3(study);
	let title_xml = write_c_5_2(study);
	let study_type = write_c_5_4(study);
	let regional_study_type =
		if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Mfds) {
			write_c_5_4_kr_1(study)
		} else {
			String::new()
		};
	let fragment = format!(
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\">{sponsor_id_xml}{study_type}{title_xml}{auth_xml}</researchStudy></subjectOf1>"
	);
	let xml = doc.to_string();
	if let Some(injected) = insert_c_5_study(&xml, &fragment) {
		let new_doc =
			parser
				.parse_string(&injected)
				.map_err(|err| Error::InvalidXml {
					message: format!("XML parse error after study injection: {err}"),
					line: None,
					column: None,
				})?;
		*doc = new_doc;
		*xpath = Context::new(doc).map_err(|_| Error::InvalidXml {
			message: "Failed to initialize XPath context after study injection"
				.to_string(),
			line: None,
			column: None,
		})?;
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		let _ = xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
		if !regional_study_type.is_empty() {
			append_fragment_child(
				doc,
				parser,
				xpath,
				"//hl7:investigationEvent",
				&regional_study_type,
			)?;
		}
	}
	Ok(())
}

/// e2b:C.5.1.r.1
fn write_c_5_1_r_1(value: &StudyRegistrationNumber) -> String {
	let Some(registration_number) = value
		.registration_number
		.as_deref()
		.filter(|value| !value.trim().is_empty())
	else {
		return value
			.registration_number_null_flavor
			.as_deref()
			.map(|null_flavor| {
				format!(
					"<id nullFlavor=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.6\"/>",
					xml_escape(null_flavor)
				)
			})
			.unwrap_or_default();
	};
	format!(
		"<id extension=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.6\"/>",
		xml_escape(registration_number)
	)
}

/// e2b:C.5.1.r.2
fn write_c_5_1_r_2(value: &StudyRegistrationNumber) -> String {
	match (value.country_code.as_deref().filter(|v| !v.trim().is_empty()), value.country_code_null_flavor.as_deref()) {
		(Some(code), _) => format!("<author typeCode=\"AUT\"><territorialAuthority classCode=\"TERR\"><governingPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code code=\"{}\" codeSystem=\"1.0.3166.1.2.2\"/></governingPlace></territorialAuthority></author>", xml_escape(code)),
		(None, Some(null_flavor)) => format!("<author typeCode=\"AUT\"><territorialAuthority classCode=\"TERR\"><governingPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code nullFlavor=\"{}\" codeSystem=\"1.0.3166.1.2.2\"/></governingPlace></territorialAuthority></author>", xml_escape(null_flavor)),
		(None, None) => String::new(),
	}
}

/// e2b:C.5.2
fn write_c_5_2(value: &StudyInformation) -> String {
	let name = value.study_name.as_deref().unwrap_or("").trim();
	if !name.is_empty() {
		return format!("<title>{}</title>", xml_escape(name));
	}
	value
		.study_name_null_flavor
		.as_deref()
		.map(|v| format!("<title nullFlavor=\"{}\"/>", xml_escape(v)))
		.unwrap_or_else(|| "<title/>".to_string())
}

/// e2b:C.5.3
fn write_c_5_3(value: &StudyInformation) -> String {
	let number = value.sponsor_study_number.as_deref().unwrap_or("").trim();
	if !number.is_empty() {
		return format!(
			"<id extension=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.5\"/>",
			xml_escape(number)
		);
	}
	value
		.sponsor_study_number_null_flavor
		.as_deref()
		.map(|null_flavor| {
			format!(
				"<id nullFlavor=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.5\"/>",
				xml_escape(null_flavor)
			)
		})
		.unwrap_or_default()
}

/// e2b:C.5.4
fn write_c_5_4(value: &StudyInformation) -> String {
	value
		.study_type_reaction
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| {
			format!(
				"<code code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.8\"/>",
				xml_escape(value)
			)
		})
		.unwrap_or_default()
}

/// e2b:C.5.4.KR.1
fn write_c_5_4_kr_1(value: &StudyInformation) -> String {
	value.study_type_reaction_kr1.as_deref().filter(|v| !v.trim().is_empty()).map(|v| format!("<subjectOf2 typeCode=\"SUBJ\"><investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.7\" codeSystemVersion=\"1.0\" displayName=\"otherStudiesType\"/><value xsi:type=\"CE\" code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.3\" codeSystemVersion=\"1.0\"/></investigationCharacteristic></subjectOf2>", xml_escape(v))).unwrap_or_default()
}

/// e2b:FDA.C.5.5a
fn write_fda_c_5_5a(value: &StudyInformation) -> String {
	write_fda_study_id(
		value.fda_ind_number_occurred.as_deref(),
		"2.16.840.1.113883.3.989.5.1.2.2.1.2.1",
	)
}

/// e2b:FDA.C.5.5b
fn write_fda_c_5_5b(value: &StudyInformation) -> String {
	write_fda_study_id(
		value.fda_pre_anda_number_occurred.as_deref(),
		"2.16.840.1.113883.3.989.5.1.2.2.1.2.2",
	)
}

/// e2b:FDA.C.5.6.r
fn write_fda_c_5_6_r(value: &StudyFdaCrossReportedInd) -> String {
	if let Some(number) =
		value.ind_number.as_deref().filter(|v| !v.trim().is_empty())
	{
		format!(
			"<id extension=\"{}\" root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.3\"/>",
			xml_escape(number)
		)
	} else if let Some(null_flavor) = value.ind_number_null_flavor.as_deref() {
		format!(
			"<id nullFlavor=\"{}\" root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.3\"/>",
			xml_escape(null_flavor)
		)
	} else {
		String::new()
	}
}

fn write_fda_study_id(value: Option<&str>, root: &str) -> String {
	value
		.filter(|v| !v.trim().is_empty())
		.map(|v| format!("<id extension=\"{}\" root=\"{root}\"/>", xml_escape(v)))
		.unwrap_or_default()
}

pub(crate) async fn fetch_study_information(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<StudyInformation>> {
	let mut studies = StudyInformationBmc::list(
		ctx,
		mm,
		Some(vec![StudyInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	studies.sort_by_key(|study| study.created_at);
	Ok(studies.into_iter().next())
}

pub(crate) async fn fetch_literature_references(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Vec<LiteratureReference>> {
	let sql = "SELECT * FROM literature_references WHERE case_id = $1 AND deleted = false ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, LiteratureReference>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(lib_core::model::Error::Store(format!("{e}"))))
}

pub(crate) async fn fetch_study_registrations(
	ctx: &Ctx,
	mm: &ModelManager,
	study_information_id: sqlx::types::Uuid,
) -> Result<Vec<StudyRegistrationNumber>> {
	let mut registrations = StudyRegistrationNumberBmc::list(
		ctx,
		mm,
		Some(vec![StudyRegistrationNumberFilter {
			study_information_id: Some(uuid_eq(study_information_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	registrations.sort_by_key(|value| value.sequence_number);
	Ok(registrations)
}

pub(crate) async fn fetch_study_fda_cross_reported_inds(
	ctx: &Ctx,
	mm: &ModelManager,
	study_information_id: sqlx::types::Uuid,
) -> Result<Vec<StudyFdaCrossReportedInd>> {
	let mut values = StudyFdaCrossReportedIndBmc::list(
		ctx,
		mm,
		Some(vec![StudyFdaCrossReportedIndFilter {
			study_information_id: Some(uuid_eq(study_information_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	values.sort_by_key(|value| value.sequence_number);
	Ok(values)
}

#[cfg(test)]
mod study_writer_strictness_tests {
	use super::*;
	use sqlx::types::time::OffsetDateTime;

	fn study() -> StudyInformation {
		StudyInformation {
			id: sqlx::types::Uuid::nil(),
			case_id: sqlx::types::Uuid::nil(),
			source_study_presave_id: None,
			study_name: None,
			study_name_null_flavor: None,
			sponsor_study_number: None,
			sponsor_study_number_null_flavor: None,
			study_type_reaction: None,
			study_type_reaction_kr1: None,
			fda_ind_number_occurred: None,
			fda_pre_anda_number_occurred: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: sqlx::types::Uuid::nil(),
			updated_by: None,
		}
	}

	#[test]
	fn mfds_study_type_uses_the_investigation_event_shape() {
		let mut study = study();
		study.study_type_reaction_kr1 = Some("4".to_string());

		let xml = write_c_5_4_kr_1(&study);
		assert!(xml.contains("<investigationCharacteristic"));
		assert!(xml.contains("codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.7\""));
		assert!(!xml.contains("<observation"));
	}

	fn registration() -> StudyRegistrationNumber {
		StudyRegistrationNumber {
			id: sqlx::types::Uuid::nil(),
			study_information_id: sqlx::types::Uuid::nil(),
			registration_number: Some(String::new()),
			registration_number_null_flavor: None,
			country_code: None,
			country_code_null_flavor: None,
			sequence_number: 1,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: sqlx::types::Uuid::nil(),
			updated_by: None,
		}
	}

	#[test]
	fn study_writers_omit_missing_values_and_keep_explicit_null_flavors() {
		let mut registration = registration();
		assert!(write_c_5_1_r_1(&registration).is_empty());
		registration.registration_number_null_flavor = Some("ASKU".to_string());
		assert!(write_c_5_1_r_1(&registration).contains("nullFlavor=\"ASKU\""));

		let mut study = study();
		assert!(write_c_5_3(&study).is_empty());
		study.sponsor_study_number_null_flavor = Some("NASK".to_string());
		assert!(write_c_5_3(&study).contains("nullFlavor=\"NASK\""));
		assert!(write_c_5_4(&study).is_empty());
		study.study_type_reaction = Some("1".to_string());
		assert_eq!(
			write_c_5_4(&study),
			"<code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.8\"/>"
		);
	}
}

fn insert_c_5_study(xml: &str, fragment: &str) -> Option<String> {
	let primary_start = xml.find("<primaryRole")?;
	let primary_end = xml[primary_start..].find("</primaryRole>")? + primary_start;
	let body_start = xml[primary_start..].find('>')? + primary_start + 1;
	let body = &xml[body_start..primary_end];
	let insert_at = body
		.find("<subjectOf2")
		.map(|idx| body_start + idx)
		.unwrap_or(primary_end);
	let mut out = String::with_capacity(xml.len() + fragment.len() + 8);
	out.push_str(&xml[..insert_at]);
	out.push_str(fragment);
	out.push_str(&xml[insert_at..]);
	Some(out)
}

fn insert_c_1_or_c_4(xml: &str, fragment: &str) -> Option<String> {
	let start = xml.find("<investigationEvent")?;
	let end = xml[start..].find("</investigationEvent>")? + start;
	let body_start = xml[start..].find('>')? + start + 1;
	let body = &xml[body_start..end];
	let insert_at = body
		.find("<component")
		.map(|idx| body_start + idx)
		.unwrap_or(end);
	let mut out = String::with_capacity(xml.len() + fragment.len() + 8);
	out.push_str(&xml[..insert_at]);
	out.push_str(fragment);
	out.push_str(&xml[insert_at..]);
	Some(out)
}

#[cfg(test)]
mod relationship_order_tests {
	use super::*;
	use libxml::tree::NodeType;
	use sqlx::types::time::OffsetDateTime;

	#[test]
	fn previous_identifier_is_exported_after_components() {
		let identifier = OtherCaseIdentifier {
			id: sqlx::types::Uuid::nil(),
			case_id: sqlx::types::Uuid::nil(),
			sequence_number: 1,
			source_of_identifier: "QVIS Safety CRO".to_string(),
			case_identifier: "KR-QVIS-PRIOR-2026-004".to_string(),
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: sqlx::types::Uuid::nil(),
			updated_by: None,
		};
		let xml = insert_c_1_or_c_4(
			crate::export::base_export_skeleton(),
			&write_c_1_9_1(&identifier),
		)
		.expect("inject previous identifier");
		let parser = Parser::default();
		let doc = parser.parse_string(&xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		crate::export::roundtrip::reorder_investigation_event_children(&mut xpath);

		let base = "//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.3']";
		assert_eq!(
			xpath
				.findvalue(&format!("{base}/@assigningAuthorityName"), None)
				.unwrap(),
			"QVIS Safety CRO"
		);
		assert_eq!(
			xpath
				.findvalue(&format!("{base}/@extension"), None)
				.unwrap(),
			"KR-QVIS-PRIOR-2026-004"
		);

		let event = xpath
			.findnodes("//hl7:investigationEvent", None)
			.unwrap()
			.into_iter()
			.next()
			.unwrap();
		let names = event
			.get_child_nodes()
			.into_iter()
			.filter(|node| node.get_type() == Some(NodeType::ElementNode))
			.map(|node| node.get_name())
			.collect::<Vec<_>>();
		let last_component =
			names.iter().rposition(|name| name == "component").unwrap();
		let first_subject =
			names.iter().position(|name| name == "subjectOf1").unwrap();
		assert!(last_component < first_subject, "{names:?}");
	}
}
