use crate::model::receiver::ReceiverInformationForUpdate;
use crate::xml::error::Error;
use crate::xml::import_runtime::shared::{
	first_attr, first_text, first_text_root, first_value_root, normalize_code,
	normalize_iso2, telecom_first, telecom_first_in_node, MessageHeaderExtract,
};
use crate::xml::mfds::codes::KR_C_3_1_1;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::xpath::Context;

pub(crate) struct SenderImport {
	pub(crate) sender_type: String,
	pub(crate) health_professional_type_kr1: Option<String>,
	pub(crate) organization_name: String,
	pub(crate) department: Option<String>,
	pub(crate) street_address: Option<String>,
	pub(crate) city: Option<String>,
	pub(crate) state: Option<String>,
	pub(crate) postcode: Option<String>,
	pub(crate) country_code: Option<String>,
	pub(crate) person_title: Option<String>,
	pub(crate) person_given_name: Option<String>,
	pub(crate) person_middle_name: Option<String>,
	pub(crate) person_family_name: Option<String>,
	pub(crate) telephone: Option<String>,
	pub(crate) fax: Option<String>,
	pub(crate) email: Option<String>,
}

pub(crate) struct PrimarySourceImport {
	pub(crate) reporter_title: Option<String>,
	pub(crate) reporter_given_name: Option<String>,
	pub(crate) reporter_middle_name: Option<String>,
	pub(crate) reporter_family_name: Option<String>,
	pub(crate) organization: Option<String>,
	pub(crate) department: Option<String>,
	pub(crate) street: Option<String>,
	pub(crate) city: Option<String>,
	pub(crate) state: Option<String>,
	pub(crate) postcode: Option<String>,
	pub(crate) telephone: Option<String>,
	pub(crate) country_code: Option<String>,
	pub(crate) email: Option<String>,
	pub(crate) qualification: Option<String>,
	pub(crate) primary_source_regulatory: Option<String>,
}

#[derive(Debug)]
pub(crate) struct OtherCaseIdentifierImport {
	pub(crate) source_of_identifier: String,
	pub(crate) case_identifier: String,
}

#[derive(Debug)]
pub(crate) struct LinkedReportImport {
	pub(crate) linked_report_number: String,
}

#[derive(Debug)]
pub(crate) struct LiteratureImport {
	pub(crate) reference_text: String,
	pub(crate) document_base64: Option<String>,
	pub(crate) media_type: Option<String>,
	pub(crate) representation: Option<String>,
	pub(crate) compression: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DocumentHeldImport {
	pub(crate) title: Option<String>,
	pub(crate) document_base64: Option<String>,
	pub(crate) media_type: Option<String>,
	pub(crate) representation: Option<String>,
	pub(crate) compression: Option<String>,
}

#[derive(Debug)]
pub(crate) struct StudyImport {
	pub(crate) study_name: Option<String>,
	pub(crate) sponsor_study_number: Option<String>,
	pub(crate) study_type_reaction: Option<String>,
	pub(crate) registrations: Vec<StudyRegistrationImport>,
}

#[derive(Debug)]
pub(crate) struct StudyRegistrationImport {
	pub(crate) registration_number: String,
	pub(crate) country_code: Option<String>,
}

pub(crate) fn parse_sender_information(
	xml: &[u8],
	header: Option<&MessageHeaderExtract>,
) -> Result<Option<SenderImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
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

	let sender_type = normalize_code(
		first_value_root(
			&mut xpath,
			"//hl7:sender/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:code/@code",
		)
		.or_else(|| {
			first_value_root(
				&mut xpath,
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code/@code",
			)
		}),
		&["1", "2", "3", "4", "5", "6", "7"],
		"sender_information.sender_type",
	)
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.C.3.1.REQUIRED: sender type missing".to_string(),
		line: None,
		column: None,
	})?;

	let organization_name = first_text_root(
		&mut xpath,
		"//hl7:sender/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:name",
	)
	.or_else(|| {
		first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:representedOrganization/hl7:name",
		)
	})
	.or_else(|| header.and_then(|h| h.message_sender.clone()))
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.C.3.2.REQUIRED: sender organization missing".to_string(),
		line: None,
		column: None,
	})?;

	Ok(Some(SenderImport {
		sender_type,
		health_professional_type_kr1: normalize_code(
			first_value_root(
				&mut xpath,
				&format!(
					"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:subjectOf2/hl7:observation[hl7:code[@code='{KR_C_3_1_1}']]/hl7:value/@code"
				),
			),
			&["1", "2", "3", "4"],
			"sender_information.health_professional_type_kr1",
		),
		organization_name,
		department: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:representedOrganization/hl7:desc",
		),
		street_address: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
		),
		city: first_text_root(&mut xpath, "//hl7:assignedEntity/hl7:addr/hl7:city"),
		state: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:addr/hl7:state",
		),
		postcode: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:addr/hl7:postalCode",
		),
		country_code: normalize_iso2(
			first_value_root(
				&mut xpath,
				"//hl7:assignedEntity/hl7:addr/hl7:country/@code",
			),
			"sender_information.country_code",
		),
		person_title: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:prefix",
		),
		person_given_name: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given",
		),
		person_middle_name: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given[2]",
		),
		person_family_name: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:family",
		),
		telephone: telecom_first(&mut xpath, "tel:"),
		fax: telecom_first(&mut xpath, "fax:"),
		email: telecom_first(&mut xpath, "mailto:"),
	}))
}

pub(crate) fn parse_primary_sources(xml: &[u8]) -> Result<Vec<PrimarySourceImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
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
	let nodes = xpath
		.findnodes(
			"//hl7:outboundRelationship[@typeCode='SPRT'][hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:relatedInvestigation",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query primary sources".to_string(),
			line: None,
			column: None,
		})?;
	let mut items = Vec::new();
	for node in nodes {
		let reporter_title = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:prefix",
		);
		let reporter_given_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:given",
		);
		let reporter_middle_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:given[2]",
		);
		let reporter_family_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:family",
		);
		let organization = first_text(
			&mut xpath,
			&node,
			".//hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name",
		);
		let department =
			first_text(&mut xpath, &node, ".//hl7:representedOrganization/hl7:name");
		let street = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
		);
		let city =
			first_text(&mut xpath, &node, ".//hl7:assignedEntity/hl7:addr/hl7:city");
		let state = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:state",
		);
		let postcode = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:postalCode",
		);
		let telephone = telecom_first_in_node(&mut xpath, &node, "tel:");
		let email = telecom_first_in_node(&mut xpath, &node, "mailto:");
		let country_code = normalize_iso2(
			first_attr(
				&mut xpath,
				&node,
				".//hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code",
				"code",
			),
			"primary_sources.country_code",
		);
		let qualification = normalize_code(
			first_attr(
				&mut xpath,
				&node,
				".//hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code",
				"code",
			),
			&["1", "2", "3", "4", "5"],
			"primary_sources.qualification",
		)
		.or(Some("1".to_string()));
		let primary_source_regulatory =
			first_attr(&mut xpath, &node, "../hl7:priorityNumber", "value")
				.filter(|value| !value.trim().is_empty())
				.or(Some("2".to_string()));

		if reporter_given_name.is_none()
			&& reporter_family_name.is_none()
			&& organization.is_none()
		{
			continue;
		}

		items.push(PrimarySourceImport {
			reporter_title,
			reporter_given_name,
			reporter_middle_name,
			reporter_family_name,
			organization,
			department,
			street,
			city,
			state,
			postcode,
			telephone,
			country_code,
			email,
			qualification,
			primary_source_regulatory,
		});
	}

	Ok(items)
}

pub(crate) fn parse_other_case_identifiers(
	xml: &[u8],
) -> Result<Vec<OtherCaseIdentifierImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
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

	let nodes = xpath
		.findnodes(
			"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:id",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query other case identifiers".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let source = node.get_attribute("assigningAuthorityName");
		let extension = node.get_attribute("extension");
		let Some(source) = source else {
			continue;
		};
		let Some(case_identifier) = extension else {
			continue;
		};
		if source.trim().is_empty() || case_identifier.trim().is_empty() {
			continue;
		}
		items.push(OtherCaseIdentifierImport {
			source_of_identifier: source,
			case_identifier,
		});
	}
	Ok(items)
}

pub(crate) fn parse_linked_reports(xml: &[u8]) -> Result<Vec<LinkedReportImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
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

	let nodes = xpath
		.findnodes(
			"//hl7:investigationEvent/hl7:outboundRelationship[@typeCode='SPRT']/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:id",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query linked reports".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let extension = node.get_attribute("extension");
		let Some(linked_report_number) = extension else {
			continue;
		};
		if linked_report_number.trim().is_empty() {
			continue;
		}
		items.push(LinkedReportImport {
			linked_report_number,
		});
	}
	Ok(items)
}

pub(crate) fn parse_documents_held_by_sender(
	xml: &[u8],
) -> Result<Vec<DocumentHeldImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
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

	let nodes = xpath
		.findnodes(
			"//hl7:reference/hl7:document[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.27']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query documents held by sender".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let title = first_text(&mut xpath, &node, "hl7:title");
		let document_base64 = first_text(&mut xpath, &node, "hl7:text");
		let media_type = first_attr(&mut xpath, &node, "hl7:text", "mediaType");
		let representation =
			first_attr(&mut xpath, &node, "hl7:text", "representation");
		let compression = first_attr(&mut xpath, &node, "hl7:text", "compression");
		items.push(DocumentHeldImport {
			title,
			document_base64,
			media_type,
			representation,
			compression,
		});
	}
	Ok(items)
}

pub(crate) fn parse_literature_references(
	xml: &[u8],
) -> Result<Vec<LiteratureImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
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

	let nodes = xpath
		.findnodes(
			"//hl7:reference/hl7:document[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.27']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query literature references".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for (idx, node) in nodes.into_iter().enumerate() {
		let reference_text =
			first_text(&mut xpath, &node, "hl7:bibliographicDesignationText")
				.or_else(|| first_text(&mut xpath, &node, "hl7:title"))
				.ok_or_else(|| Error::InvalidXml {
					message: format!(
						"ICH.C.4.r.REQUIRED: literature reference text missing for sequence {}",
						idx + 1
					),
					line: None,
					column: None,
				})?;
		let document_base64 = first_text(&mut xpath, &node, "hl7:text");
		let media_type = first_attr(&mut xpath, &node, "hl7:text", "mediaType");
		let representation =
			first_attr(&mut xpath, &node, "hl7:text", "representation");
		let compression = first_attr(&mut xpath, &node, "hl7:text", "compression");
		items.push(LiteratureImport {
			reference_text,
			document_base64,
			media_type,
			representation,
			compression,
		});
	}
	Ok(items)
}

pub(crate) fn parse_study_information(xml: &[u8]) -> Result<Option<StudyImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
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

	let nodes = xpath.findnodes("//hl7:researchStudy", None).map_err(|_| {
		Error::InvalidXml {
			message: "Failed to query study information".to_string(),
			line: None,
			column: None,
		}
	})?;
	let Some(node) = nodes.get(0) else {
		return Ok(None);
	};

	let study_name = first_text(&mut xpath, node, "hl7:title");
	let sponsor_study_number = first_attr(&mut xpath, node, "hl7:id", "extension");
	let study_type_reaction = first_attr(&mut xpath, node, "hl7:code", "code");

	let reg_nodes = xpath
		.findnodes(".//hl7:studyRegistration", Some(node))
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query study registrations".to_string(),
			line: None,
			column: None,
		})?;
	let mut registrations = Vec::new();
	for reg in reg_nodes {
		let registration_number =
			first_attr(&mut xpath, &reg, "hl7:id", "extension");
		let Some(registration_number) = registration_number else {
			continue;
		};
		let country_code = first_attr(
			&mut xpath,
			&reg,
			"hl7:author/hl7:territorialAuthority/hl7:governingPlace/hl7:code",
			"code",
		);
		registrations.push(StudyRegistrationImport {
			registration_number,
			country_code: normalize_iso2(
				country_code,
				"study_registration.country_code",
			),
		});
	}

	Ok(Some(StudyImport {
		study_name,
		sponsor_study_number,
		study_type_reaction,
		registrations,
	}))
}

pub(crate) fn parse_receiver_information(
	xml: &[u8],
) -> Result<Option<ReceiverInformationForUpdate>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
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
	let receiver_node = xpath
		.findnodes("//hl7:receiver/hl7:device", None)
		.ok()
		.and_then(|nodes| nodes.into_iter().next());

	let organization_name =
		first_value_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:id/@extension")
			.or_else(|| {
				first_text_root(
			&mut xpath,
			"//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:name",
		)
			});

	if organization_name.is_none() {
		return Ok(None);
	}

	Ok(Some(ReceiverInformationForUpdate {
		receiver_type: normalize_code(
			first_value_root(
				&mut xpath,
				"//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:code/@code",
			),
			&["1", "2", "3", "4", "5", "6"],
			"receiver_information.receiver_type",
		),
		organization_name,
		department: first_text_root(
			&mut xpath,
			"//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:desc",
		),
		street_address: first_text_root(
			&mut xpath,
			"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:streetAddressLine",
		),
		city: first_text_root(
			&mut xpath,
			"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:city",
		),
		state_province: first_text_root(
			&mut xpath,
			"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:state",
		),
		postcode: first_text_root(
			&mut xpath,
			"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:postalCode",
		),
		country_code: normalize_iso2(
			first_value_root(
				&mut xpath,
				"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:country/@code",
			),
			"receiver_information.country_code",
		),
		telephone: receiver_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "tel:")),
		fax: receiver_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "fax:")),
		email: receiver_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "mailto:")),
	}))
}
