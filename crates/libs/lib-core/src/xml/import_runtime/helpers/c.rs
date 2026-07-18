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
	pub(crate) reporter_title_null_flavor: Option<String>,
	pub(crate) reporter_given_name: Option<String>,
	pub(crate) reporter_given_name_null_flavor: Option<String>,
	pub(crate) reporter_middle_name: Option<String>,
	pub(crate) reporter_middle_name_null_flavor: Option<String>,
	pub(crate) reporter_family_name: Option<String>,
	pub(crate) reporter_family_name_null_flavor: Option<String>,
	pub(crate) organization: Option<String>,
	pub(crate) organization_null_flavor: Option<String>,
	pub(crate) department: Option<String>,
	pub(crate) department_null_flavor: Option<String>,
	pub(crate) street: Option<String>,
	pub(crate) street_null_flavor: Option<String>,
	pub(crate) city: Option<String>,
	pub(crate) city_null_flavor: Option<String>,
	pub(crate) state: Option<String>,
	pub(crate) state_null_flavor: Option<String>,
	pub(crate) postcode: Option<String>,
	pub(crate) postcode_null_flavor: Option<String>,
	pub(crate) telephone: Option<String>,
	pub(crate) telephone_null_flavor: Option<String>,
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
	pub(crate) reference_text_null_flavor: Option<String>,
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
	pub(crate) study_name_null_flavor: Option<String>,
	pub(crate) sponsor_study_number: Option<String>,
	pub(crate) sponsor_study_number_null_flavor: Option<String>,
	pub(crate) study_type_reaction: Option<String>,
	pub(crate) registrations: Vec<StudyRegistrationImport>,
}

#[derive(Debug)]
pub(crate) struct StudyRegistrationImport {
	pub(crate) registration_number: String,
	pub(crate) registration_number_null_flavor: Option<String>,
	pub(crate) country_code: Option<String>,
	pub(crate) country_code_null_flavor: Option<String>,
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
	let sender_node = xpath
		.findnodes(
			"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity[hl7:code[@codeSystem='2.16.840.1.113883.3.989.2.1.1.7']]",
			None,
		)
		.ok()
		.and_then(|nodes| nodes.into_iter().next());

	let sender_type_raw = if let Some(node) = sender_node.as_ref() {
		first_attr(
			&mut xpath,
			node,
			"./hl7:code[@codeSystem='2.16.840.1.113883.3.989.2.1.1.7']",
			"code",
		)
	} else {
		first_value_root(
			&mut xpath,
			"//hl7:sender/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:code/@code",
		)
		.or_else(|| {
			first_value_root(
				&mut xpath,
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code/@code",
			)
		})
	};
	let sender_type = normalize_code(
		sender_type_raw,
		&["1", "2", "3", "4", "5", "6", "7"],
		"sender_information.sender_type",
	)
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.C.3.1.REQUIRED: sender type missing".to_string(),
		line: None,
		column: None,
	})?;

	let organization_name = sender_node
		.as_ref()
		.and_then(|node| {
			first_text(
				&mut xpath,
				node,
				"./hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name",
			)
		})
		.or_else(|| {
			sender_node.as_ref().and_then(|node| {
				first_text(&mut xpath, node, "./hl7:representedOrganization/hl7:name")
			})
		})
		.or_else(|| {
			first_text_root(
		&mut xpath,
		"//hl7:sender/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:name",
		)
		})
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
			if let Some(node) = sender_node.as_ref() {
				first_attr(
					&mut xpath,
					node,
					&format!("./hl7:subjectOf2/hl7:observation[hl7:code[@code='{KR_C_3_1_1}']]/hl7:value"),
					"code",
				)
			} else {
				first_value_root(
					&mut xpath,
					&format!(
						"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:subjectOf2/hl7:observation[hl7:code[@code='{KR_C_3_1_1}']]/hl7:value/@code"
					),
				)
			},
			&["1", "2", "3", "4"],
			"sender_information.health_professional_type_kr1",
		),
		organization_name,
		department: sender_node
			.as_ref()
			.and_then(|node| {
				first_text(
					&mut xpath,
					node,
					"./hl7:representedOrganization/hl7:name",
				)
			})
			.or_else(|| {
				first_text_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:representedOrganization/hl7:desc",
				)
			}),
		street_address: sender_node
			.as_ref()
			.and_then(|node| {
				first_text(&mut xpath, node, "./hl7:addr/hl7:streetAddressLine")
			})
			.or_else(|| {
				first_text_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
				)
			}),
		city: sender_node
			.as_ref()
			.and_then(|node| first_text(&mut xpath, node, "./hl7:addr/hl7:city"))
			.or_else(|| {
				first_text_root(&mut xpath, "//hl7:assignedEntity/hl7:addr/hl7:city")
			}),
		state: sender_node
			.as_ref()
			.and_then(|node| first_text(&mut xpath, node, "./hl7:addr/hl7:state"))
			.or_else(|| {
				first_text_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:addr/hl7:state",
				)
			}),
		postcode: sender_node
			.as_ref()
			.and_then(|node| {
				first_text(&mut xpath, node, "./hl7:addr/hl7:postalCode")
			})
			.or_else(|| {
				first_text_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:addr/hl7:postalCode",
				)
			}),
		country_code: normalize_iso2(
			if let Some(node) = sender_node.as_ref() {
				first_attr(
					&mut xpath,
					node,
					"./hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code",
					"code",
				)
				.or_else(|| {
					first_attr(&mut xpath, node, "./hl7:addr/hl7:country", "code")
				})
			} else {
				first_value_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:addr/hl7:country/@code",
				)
			},
			"sender_information.country_code",
		),
		person_title: sender_node
			.as_ref()
			.and_then(|node| {
				first_text(
					&mut xpath,
					node,
					"./hl7:assignedPerson/hl7:name/hl7:prefix",
				)
			})
			.or_else(|| {
				first_text_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:prefix",
				)
			}),
		person_given_name: sender_node
			.as_ref()
			.and_then(|node| {
				first_text(
					&mut xpath,
					node,
					"./hl7:assignedPerson/hl7:name/hl7:given",
				)
			})
			.or_else(|| {
				first_text_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given",
				)
			}),
		person_middle_name: sender_node
			.as_ref()
			.and_then(|node| {
				first_text(
					&mut xpath,
					node,
					"./hl7:assignedPerson/hl7:name/hl7:given[2]",
				)
			})
			.or_else(|| {
				first_text_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given[2]",
				)
			}),
		person_family_name: sender_node
			.as_ref()
			.and_then(|node| {
				first_text(
					&mut xpath,
					node,
					"./hl7:assignedPerson/hl7:name/hl7:family",
				)
			})
			.or_else(|| {
				first_text_root(
					&mut xpath,
					"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:family",
				)
			}),
		telephone: sender_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "tel:"))
			.or_else(|| telecom_first(&mut xpath, "tel:")),
		fax: sender_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "fax:"))
			.or_else(|| telecom_first(&mut xpath, "fax:")),
		email: sender_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "mailto:"))
			.or_else(|| telecom_first(&mut xpath, "mailto:")),
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
		let reporter_title_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:prefix",
			"nullFlavor",
		);
		let reporter_given_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:given",
		);
		let reporter_given_name_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:given[1]",
			"nullFlavor",
		);
		let reporter_middle_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:given[2]",
		);
		let reporter_middle_name_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:given[2]",
			"nullFlavor",
		);
		let reporter_family_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:family",
		);
		let reporter_family_name_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:family",
			"nullFlavor",
		);
		let nested_organization = first_text(
			&mut xpath,
			&node,
			".//hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name",
		);
		let nested_organization_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name",
			"nullFlavor",
		);
		let direct_organization =
			first_text(&mut xpath, &node, ".//hl7:representedOrganization/hl7:name");
		let direct_organization_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:representedOrganization/hl7:name",
			"nullFlavor",
		);
		let has_nested_organization = nested_organization.is_some()
			|| nested_organization_null_flavor.is_some();
		let organization = nested_organization
			.clone()
			.or_else(|| direct_organization.clone());
		let organization_null_flavor =
			nested_organization_null_flavor.clone().or_else(|| {
				(!has_nested_organization)
					.then_some(direct_organization_null_flavor.clone())
					.flatten()
			});
		let department = if has_nested_organization {
			direct_organization.clone()
		} else {
			None
		};
		let department_null_flavor = has_nested_organization
			.then_some(direct_organization_null_flavor)
			.flatten();
		let street = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
		);
		let street_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
			"nullFlavor",
		);
		let city =
			first_text(&mut xpath, &node, ".//hl7:assignedEntity/hl7:addr/hl7:city");
		let city_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:city",
			"nullFlavor",
		);
		let state = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:state",
		);
		let state_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:state",
			"nullFlavor",
		);
		let postcode = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:postalCode",
		);
		let postcode_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:postalCode",
			"nullFlavor",
		);
		let telephone = telecom_first_in_node(&mut xpath, &node, "tel:");
		let telephone_null_flavor = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:telecom[not(starts-with(@value,'mailto:'))][1]",
			"nullFlavor",
		);
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
		let qualification_raw = first_attr(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code",
			"code",
		);
		let qualification = normalize_code(
			qualification_raw.clone(),
			&["1", "2", "3", "4", "5"],
			"primary_sources.qualification",
		)
		.or(Some("1".to_string()));
		let primary_source_regulatory_raw =
			first_attr(&mut xpath, &node, "../hl7:priorityNumber", "value")
				.filter(|value| !value.trim().is_empty());
		let primary_source_regulatory = primary_source_regulatory_raw
			.clone()
			.or(Some("2".to_string()));

		let has_importable_content = [
			reporter_title.as_ref(),
			reporter_title_null_flavor.as_ref(),
			reporter_given_name.as_ref(),
			reporter_given_name_null_flavor.as_ref(),
			reporter_middle_name.as_ref(),
			reporter_middle_name_null_flavor.as_ref(),
			reporter_family_name.as_ref(),
			reporter_family_name_null_flavor.as_ref(),
			organization.as_ref(),
			organization_null_flavor.as_ref(),
			department.as_ref(),
			department_null_flavor.as_ref(),
			street.as_ref(),
			street_null_flavor.as_ref(),
			city.as_ref(),
			city_null_flavor.as_ref(),
			state.as_ref(),
			state_null_flavor.as_ref(),
			postcode.as_ref(),
			postcode_null_flavor.as_ref(),
			telephone.as_ref(),
			telephone_null_flavor.as_ref(),
			country_code.as_ref(),
			email.as_ref(),
			qualification_raw.as_ref(),
			primary_source_regulatory_raw.as_ref(),
		]
		.into_iter()
		.any(|value| value.is_some());

		if !has_importable_content {
			continue;
		}

		items.push(PrimarySourceImport {
			reporter_title,
			reporter_title_null_flavor,
			reporter_given_name,
			reporter_given_name_null_flavor,
			reporter_middle_name,
			reporter_middle_name_null_flavor,
			reporter_family_name,
			reporter_family_name_null_flavor,
			organization,
			organization_null_flavor,
			department,
			department_null_flavor,
			street,
			street_null_flavor,
			city,
			city_null_flavor,
			state,
			state_null_flavor,
			postcode,
			postcode_null_flavor,
			telephone,
			telephone_null_flavor,
			country_code,
			email,
			qualification,
			primary_source_regulatory,
		});
	}

	Ok(items)
}

#[cfg(test)]
mod tests {
	use super::parse_primary_sources;

	fn primary_source_xml(body: &str) -> String {
		format!(
			r#"<?xml version="1.0" encoding="utf-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3">
  <PORR_IN049016UV>
    <controlActProcess>
      <subject>
        <investigationEvent>
          <outboundRelationship typeCode="SPRT">
            <priorityNumber value="1"/>
            <relatedInvestigation>
              <code code="2"/>
              <subjectOf2>
                <controlActEvent>
                  <author>
                    <assignedEntity>
                      {body}
                    </assignedEntity>
                  </author>
                </controlActEvent>
              </subjectOf2>
            </relatedInvestigation>
          </outboundRelationship>
        </investigationEvent>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>"#
		)
	}

	#[test]
	fn primary_source_import_reads_direct_represented_organization_name() {
		let xml = primary_source_xml(
			r#"<representedOrganization>
  <name>Direct Reporter Org</name>
</representedOrganization>"#,
		);

		let primary_sources = parse_primary_sources(xml.as_bytes()).expect("parse");

		assert_eq!(primary_sources.len(), 1);
		assert_eq!(
			primary_sources[0].organization.as_deref(),
			Some("Direct Reporter Org")
		);
	}

	#[test]
	fn primary_source_import_keeps_rows_with_contact_data_only() {
		let xml = primary_source_xml(
			r#"<addr>
  <streetAddressLine>13 Elm St.</streetAddressLine>
  <city>Metropolis</city>
</addr>
<telecom value="mailto:reporter@example.test"/>"#,
		);

		let primary_sources = parse_primary_sources(xml.as_bytes()).expect("parse");

		assert_eq!(primary_sources.len(), 1);
		assert_eq!(primary_sources[0].street.as_deref(), Some("13 Elm St."));
		assert_eq!(
			primary_sources[0].email.as_deref(),
			Some("reporter@example.test")
		);
	}

	#[test]
	fn primary_source_import_isolates_element_null_flavors() {
		let xml = primary_source_xml(
			r#"<assignedPerson><name>
  <prefix/>
  <given nullFlavor="ASKU"/>
  <family/>
</name></assignedPerson>
<addr><city nullFlavor="NASK"/><state/></addr>"#,
		);

		let primary_sources = parse_primary_sources(xml.as_bytes()).expect("parse");

		assert_eq!(primary_sources.len(), 1);
		assert_eq!(
			primary_sources[0]
				.reporter_given_name_null_flavor
				.as_deref(),
			Some("ASKU")
		);
		assert_eq!(primary_sources[0].city_null_flavor.as_deref(), Some("NASK"));
		assert!(primary_sources[0].reporter_title_null_flavor.is_none());
		assert!(primary_sources[0].state_null_flavor.is_none());
	}
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
		let reference_text_null_flavor = first_attr(
			&mut xpath,
			&node,
			"hl7:bibliographicDesignationText",
			"nullFlavor",
		)
		.or_else(|| first_attr(&mut xpath, &node, "hl7:title", "nullFlavor"));
		let reference_text =
			first_text(&mut xpath, &node, "hl7:bibliographicDesignationText")
				.or_else(|| first_text(&mut xpath, &node, "hl7:title"))
				.or_else(|| {
					reference_text_null_flavor.as_ref().map(|_| String::new())
				})
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
			reference_text_null_flavor,
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
	let study_name_null_flavor =
		first_attr(&mut xpath, node, "hl7:title", "nullFlavor");
	let sponsor_study_number = first_attr(&mut xpath, node, "hl7:id", "extension");
	let sponsor_study_number_null_flavor =
		first_attr(&mut xpath, node, "hl7:id", "nullFlavor");
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
		let registration_number_null_flavor =
			first_attr(&mut xpath, &reg, "hl7:id", "nullFlavor");
		let Some(registration_number) = registration_number.or_else(|| {
			registration_number_null_flavor
				.as_ref()
				.map(|_| String::new())
		}) else {
			continue;
		};
		let country_code = first_attr(
			&mut xpath,
			&reg,
			"hl7:author/hl7:territorialAuthority/hl7:governingPlace/hl7:code",
			"code",
		);
		let country_code_null_flavor = first_attr(
			&mut xpath,
			&reg,
			"hl7:author/hl7:territorialAuthority/hl7:governingPlace/hl7:code",
			"nullFlavor",
		);
		registrations.push(StudyRegistrationImport {
			registration_number,
			registration_number_null_flavor,
			country_code: normalize_iso2(
				country_code,
				"study_registration.country_code",
			),
			country_code_null_flavor,
		});
	}

	Ok(Some(StudyImport {
		study_name,
		study_name_null_flavor,
		sponsor_study_number,
		sponsor_study_number_null_flavor,
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
