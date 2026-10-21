use crate::error::Error;
use crate::import_constraint;
use crate::import_sections::shared::{
	first_attr, first_text, first_text_root, first_value_root, normalize_code,
	normalize_iso2, telecom_first_in_node,
};
use crate::mapping::mfds::c_safety_report::CMfdsSafetyReportPaths;
use crate::Result;
use lib_core::model::receiver::ReceiverInformationForUpdate;
use libxml::parser::Parser;
use libxml::tree::NodeType;
use libxml::xpath::Context;

pub(crate) struct SenderImport {
	pub(crate) sender_type: Option<String>,
	pub(crate) health_professional_type_kr1: Option<String>,
	pub(crate) organization_name: Option<String>,
	pub(crate) department: Option<String>,
	pub(crate) street_address: Option<String>,
	pub(crate) city: Option<String>,
	pub(crate) state: Option<String>,
	pub(crate) postcode: Option<String>,
	pub(crate) country_code: Option<String>,
	pub(crate) person_title: Option<String>,
	pub(crate) person_title_null_flavor: Option<String>,
	pub(crate) person_given_name: Option<String>,
	pub(crate) person_given_name_null_flavor: Option<String>,
	pub(crate) person_middle_name: Option<String>,
	pub(crate) person_middle_name_null_flavor: Option<String>,
	pub(crate) person_family_name: Option<String>,
	pub(crate) person_family_name_null_flavor: Option<String>,
	pub(crate) telephone: Option<String>,
	pub(crate) fax: Option<String>,
	pub(crate) email: Option<String>,
}

#[derive(Debug)]
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
	pub(crate) email_null_flavor: Option<String>,
	pub(crate) qualification: Option<String>,
	pub(crate) qualification_null_flavor: Option<String>,
	pub(crate) qualification_kr1: Option<String>,
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
	pub(crate) reference_text: Option<String>,
	pub(crate) reference_text_null_flavor: Option<String>,
	pub(crate) document_base64: Option<String>,
	pub(crate) file_name: Option<String>,
	pub(crate) media_type: Option<String>,
	pub(crate) representation: Option<String>,
	pub(crate) compression: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DocumentHeldImport {
	pub(crate) title: Option<String>,
	pub(crate) document_base64: Option<String>,
	pub(crate) file_name: Option<String>,
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
	pub(crate) study_type_reaction_kr1: Option<String>,
	pub(crate) fda_ind_number_occurred: Option<String>,
	pub(crate) fda_pre_anda_number_occurred: Option<String>,
	pub(crate) registrations: Vec<StudyRegistrationImport>,
	pub(crate) cross_reported_inds: Vec<(Option<String>, Option<String>)>,
}

#[derive(Debug)]
pub(crate) struct StudyRegistrationImport {
	pub(crate) registration_number: String,
	pub(crate) registration_number_null_flavor: Option<String>,
	pub(crate) country_code: Option<String>,
	pub(crate) country_code_null_flavor: Option<String>,
}

fn sender_text(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
	relative: &str,
) -> Option<String> {
	node.and_then(|node| first_text(xpath, node, relative))
}

fn sender_string(
	value: Option<String>,
	field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<Option<String>> {
	import_constraint::string(field, value.as_deref(), None, check)?;
	Ok(value)
}

fn sender_string_with_null_flavor(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
	path: &str,
	field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<(Option<String>, Option<String>)> {
	let value = sender_text(xpath, node, path);
	let null_flavor =
		node.and_then(|node| first_attr(xpath, node, path, "nullFlavor"));
	import_constraint::string(
		field,
		value.as_deref(),
		null_flavor.as_deref(),
		check,
	)?;
	Ok((value, null_flavor))
}

/// e2b:C.3.1
fn read_c_3_1(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	let raw = node.and_then(|node| {
		first_attr(
			xpath,
			node,
			"./hl7:code[@codeSystem='2.16.840.1.113883.3.989.2.1.1.7']",
			"code",
		)
	});
	import_constraint::string(
		"senderType",
		raw.as_deref(),
		None,
		input_contracts::generated::c::c_3_1,
	)?;
	Ok(raw)
}

/// e2b:C.3.1.KR.1
fn read_c_3_1_kr_1(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	let raw = node.and_then(|node| {
		first_attr(
			xpath,
			node,
			"./hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.10.1.2']",
			"code",
		)
	});
	sender_string(
		raw,
		"healthProfessionalTypeKr1",
		input_contracts::generated::c::mfds_c_3_1_kr_1,
	)
}

/// e2b:C.3.2
fn read_c_3_2(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	let value = node.and_then(|node| first_text(xpath, node, "./hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name"));
	import_constraint::string(
		"organizationName",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_3_2,
	)?;
	Ok(value)
}

/// e2b:C.3.3.1
fn read_c_3_3_1(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	sender_string(
		sender_text(xpath, node, "./hl7:representedOrganization/hl7:name"),
		"department",
		input_contracts::generated::c::c_3_3_1,
	)
}

/// e2b:C.3.3.2
fn read_c_3_3_2(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<(Option<String>, Option<String>)> {
	sender_string_with_null_flavor(
		xpath,
		node,
		"./hl7:assignedPerson/hl7:name/hl7:prefix",
		"personTitle",
		input_contracts::generated::c::c_3_3_2,
	)
}

/// e2b:C.3.3.3
fn read_c_3_3_3(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<(Option<String>, Option<String>)> {
	sender_string_with_null_flavor(
		xpath,
		node,
		"./hl7:assignedPerson/hl7:name/hl7:given[1]",
		"personGivenName",
		input_contracts::generated::c::c_3_3_3,
	)
}

/// e2b:C.3.3.4
fn read_c_3_3_4(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<(Option<String>, Option<String>)> {
	sender_string_with_null_flavor(
		xpath,
		node,
		"./hl7:assignedPerson/hl7:name/hl7:given[2]",
		"personMiddleName",
		input_contracts::generated::c::c_3_3_4,
	)
}

/// e2b:C.3.3.5
fn read_c_3_3_5(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<(Option<String>, Option<String>)> {
	sender_string_with_null_flavor(
		xpath,
		node,
		"./hl7:assignedPerson/hl7:name/hl7:family",
		"personFamilyName",
		input_contracts::generated::c::c_3_3_5,
	)
}

fn read_sender_address(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
	element: &str,
) -> Option<String> {
	sender_text(xpath, node, &format!("./hl7:addr/hl7:{element}"))
}

/// e2b:C.3.4.1
fn read_c_3_4_1(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	sender_string(
		read_sender_address(xpath, node, "streetAddressLine"),
		"streetAddress",
		input_contracts::generated::c::c_3_4_1,
	)
}
/// e2b:C.3.4.2
fn read_c_3_4_2(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	sender_string(
		read_sender_address(xpath, node, "city"),
		"city",
		input_contracts::generated::c::c_3_4_2,
	)
}
/// e2b:C.3.4.3
fn read_c_3_4_3(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	sender_string(
		read_sender_address(xpath, node, "state"),
		"state",
		input_contracts::generated::c::c_3_4_3,
	)
}
/// e2b:C.3.4.4
fn read_c_3_4_4(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	sender_string(
		read_sender_address(xpath, node, "postalCode"),
		"postcode",
		input_contracts::generated::c::c_3_4_4,
	)
}

/// e2b:C.3.4.5
fn read_c_3_4_5(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	let raw = node.and_then(|node| {
		first_attr(
			xpath,
			node,
			"./hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code",
			"code",
		)
	});
	sender_string(
		raw.map(|value| value.to_ascii_uppercase()),
		"countryCode",
		input_contracts::generated::c::c_3_4_5,
	)
}

fn read_sender_telecom(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
	prefix: &str,
) -> Option<String> {
	node.and_then(|node| telecom_first_in_node(xpath, node, prefix))
}

/// e2b:C.3.4.6
fn read_c_3_4_6(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	sender_string(
		read_sender_telecom(xpath, node, "tel:"),
		"telephone",
		input_contracts::generated::c::c_3_4_6,
	)
}
/// e2b:C.3.4.7
fn read_c_3_4_7(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	sender_string(
		read_sender_telecom(xpath, node, "fax:"),
		"fax",
		input_contracts::generated::c::c_3_4_7,
	)
}
/// e2b:C.3.4.8
fn read_c_3_4_8(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Result<Option<String>> {
	sender_string(
		read_sender_telecom(xpath, node, "mailto:"),
		"email",
		input_contracts::generated::c::c_3_4_8,
	)
}

pub(crate) fn parse_sender_information(xml: &[u8]) -> Result<Option<SenderImport>> {
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
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query sender information".to_string(),
			line: None,
			column: None,
		})?
		.into_iter()
		.next();

	let sender_type = read_c_3_1(&mut xpath, sender_node.as_ref())?;
	let organization_name = read_c_3_2(&mut xpath, sender_node.as_ref())?;
	let (person_title, person_title_null_flavor) =
		read_c_3_3_2(&mut xpath, sender_node.as_ref())?;
	let (person_given_name, person_given_name_null_flavor) =
		read_c_3_3_3(&mut xpath, sender_node.as_ref())?;
	let (person_middle_name, person_middle_name_null_flavor) =
		read_c_3_3_4(&mut xpath, sender_node.as_ref())?;
	let (person_family_name, person_family_name_null_flavor) =
		read_c_3_3_5(&mut xpath, sender_node.as_ref())?;

	Ok(Some(SenderImport {
		sender_type,
		health_professional_type_kr1: read_c_3_1_kr_1(
			&mut xpath,
			sender_node.as_ref(),
		)?,
		organization_name,
		department: read_c_3_3_1(&mut xpath, sender_node.as_ref())?,
		person_title,
		person_title_null_flavor,
		person_given_name,
		person_given_name_null_flavor,
		person_middle_name,
		person_middle_name_null_flavor,
		person_family_name,
		person_family_name_null_flavor,
		street_address: read_c_3_4_1(&mut xpath, sender_node.as_ref())?,
		city: read_c_3_4_2(&mut xpath, sender_node.as_ref())?,
		state: read_c_3_4_3(&mut xpath, sender_node.as_ref())?,
		postcode: read_c_3_4_4(&mut xpath, sender_node.as_ref())?,
		country_code: read_c_3_4_5(&mut xpath, sender_node.as_ref())?,
		telephone: read_c_3_4_6(&mut xpath, sender_node.as_ref())?,
		fax: read_c_3_4_7(&mut xpath, sender_node.as_ref())?,
		email: read_c_3_4_8(&mut xpath, sender_node.as_ref())?,
	}))
}

fn read_reporter_text_with_null_flavor(
	xpath: &mut Context,
	node: &libxml::tree::Node,
	path: &str,
	field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<(Option<String>, Option<String>)> {
	let value = first_text(xpath, node, path);
	let null_flavor = first_attr(xpath, node, path, "nullFlavor");
	import_constraint::string(
		field,
		value.as_deref(),
		null_flavor.as_deref(),
		check,
	)?;
	Ok((value, null_flavor))
}

fn read_text_with_null_flavor(
	xpath: &mut Context,
	node: &libxml::tree::Node,
	path: &str,
) -> (Option<String>, Option<String>) {
	(
		first_text(xpath, node, path),
		first_attr(xpath, node, path, "nullFlavor"),
	)
}

fn input_pair(
	field: &str,
	value: Option<String>,
	null_flavor: Option<String>,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<(Option<String>, Option<String>)> {
	import_constraint::string(
		field,
		value.as_deref(),
		null_flavor.as_deref(),
		check,
	)?;
	Ok((value, null_flavor))
}

/// e2b:C.2.r.1.1
/// e2b:C.2.r.local.reporterTitleNullFlavor
fn read_c_2_r_1_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_reporter_text_with_null_flavor(
		xpath,
		node,
		".//hl7:assignedPerson/hl7:name/hl7:prefix",
		"reporterTitle",
		input_contracts::generated::c::c_2_r_1_1,
	)
}

/// e2b:C.2.r.1.2
/// e2b:C.2.r.local.reporterGivenNameNullFlavor
fn read_c_2_r_1_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_reporter_text_with_null_flavor(
		xpath,
		node,
		".//hl7:assignedPerson/hl7:name/hl7:given[1]",
		"reporterGivenName",
		input_contracts::generated::c::c_2_r_1_2,
	)
}

/// e2b:C.2.r.1.3
/// e2b:C.2.r.local.reporterMiddleNameNullFlavor
fn read_c_2_r_1_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_reporter_text_with_null_flavor(
		xpath,
		node,
		".//hl7:assignedPerson/hl7:name/hl7:given[2]",
		"reporterMiddleName",
		input_contracts::generated::c::c_2_r_1_3,
	)
}

/// e2b:C.2.r.1.4
/// e2b:C.2.r.local.reporterFamilyNameNullFlavor
fn read_c_2_r_1_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_reporter_text_with_null_flavor(
		xpath,
		node,
		".//hl7:assignedPerson/hl7:name/hl7:family",
		"reporterFamilyName",
		input_contracts::generated::c::c_2_r_1_4,
	)
}

/// e2b:C.2.r.2.1
/// e2b:C.2.r.2.2
/// e2b:C.2.r.local.reporterOrganizationNullFlavor
/// e2b:C.2.r.local.reporterDepartmentNullFlavor
fn read_c_2_r_2_1_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
)> {
	let nested_path = "hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name";
	let direct_path = "hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:representedOrganization/hl7:name";
	let (nested, nested_null_flavor) = read_reporter_text_with_null_flavor(
		xpath,
		node,
		nested_path,
		"reporterOrganization",
		input_contracts::generated::c::c_2_r_2_1,
	)?;
	let (direct, direct_null_flavor) = read_reporter_text_with_null_flavor(
		xpath,
		node,
		direct_path,
		"reporterDepartment",
		input_contracts::generated::c::c_2_r_2_2,
	)?;
	Ok((nested, nested_null_flavor, direct, direct_null_flavor))
}

/// e2b:C.2.r.2.3
/// e2b:C.2.r.local.reporterStreetNullFlavor
fn read_c_2_r_2_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_reporter_text_with_null_flavor(
		xpath,
		node,
		".//hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
		"reporterStreet",
		input_contracts::generated::c::c_2_r_2_3,
	)
}

/// e2b:C.2.r.2.4
/// e2b:C.2.r.local.reporterCityNullFlavor
fn read_c_2_r_2_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_reporter_text_with_null_flavor(
		xpath,
		node,
		".//hl7:assignedEntity/hl7:addr/hl7:city",
		"reporterCity",
		input_contracts::generated::c::c_2_r_2_4,
	)
}

/// e2b:C.2.r.2.5
/// e2b:C.2.r.local.reporterStateNullFlavor
fn read_c_2_r_2_5(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_reporter_text_with_null_flavor(
		xpath,
		node,
		".//hl7:assignedEntity/hl7:addr/hl7:state",
		"reporterState",
		input_contracts::generated::c::c_2_r_2_5,
	)
}

/// e2b:C.2.r.2.6
/// e2b:C.2.r.local.reporterPostcodeNullFlavor
fn read_c_2_r_2_6(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_reporter_text_with_null_flavor(
		xpath,
		node,
		".//hl7:assignedEntity/hl7:addr/hl7:postalCode",
		"reporterPostcode",
		input_contracts::generated::c::c_2_r_2_6,
	)
}

/// e2b:C.2.r.2.7
/// e2b:C.2.r.local.reporterTelephoneNullFlavor
fn read_c_2_r_2_7(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let value = telecom_first_in_node(xpath, node, "tel:");
	let null_flavor = first_attr(
		xpath,
		node,
		".//hl7:assignedEntity/hl7:telecom[not(@value='mailto' or starts-with(@value,'mailto:'))][1]",
		"nullFlavor",
	);
	import_constraint::string(
		"reporterTelephone",
		value.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::c::c_2_r_2_7,
	)?;
	import_constraint::string(
		"reporterTelephoneNullFlavor",
		None,
		None,
		input_contracts::generated::c::c_2_r_2_7,
	)?;
	Ok((value, null_flavor))
}

/// e2b:FDA.C.2.r.2.8
/// e2b:C.2.r.local.reporterEmailNullFlavor
fn read_fda_c_2_r_2_8(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let value = telecom_first_in_node(xpath, node, "mailto:");
	let null_flavor = first_attr(
		xpath,
		node,
		".//hl7:assignedEntity/hl7:telecom[@value='mailto' or starts-with(@value,'mailto:')][1]",
		"nullFlavor",
	);
	import_constraint::string(
		"reporterEmail",
		value.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::c::fda_c_2_r_2_8,
	)?;
	import_constraint::string(
		"reporterEmailNullFlavor",
		None,
		None,
		input_contracts::generated::c::fda_c_2_r_2_8,
	)?;
	Ok((value, null_flavor))
}

/// e2b:C.2.r.3
fn read_c_2_r_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let value = first_attr(xpath, node, "../hl7:priorityNumber", "value")
		.filter(|value| !value.trim().is_empty());
	import_constraint::string(
		"primarySourceForRegulatoryPurposes",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_2_r_5,
	)?;
	Ok(value)
}

/// e2b:C.2.r.4
/// e2b:C.2.r.local.qualificationNullFlavor
fn read_c_2_r_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let path = ".//hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code[@codeSystem='2.16.840.1.113883.3.989.2.1.1.6' or @nullFlavor]";
	let value = first_attr(xpath, node, path, "code");
	let null_flavor = first_attr(xpath, node, path, "nullFlavor");
	import_constraint::string(
		"qualification",
		value.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::c::c_2_r_4,
	)?;
	import_constraint::string(
		"qualificationNullFlavor",
		None,
		None,
		input_contracts::generated::c::c_2_r_4,
	)?;
	Ok((value, null_flavor))
}

/// e2b:C.2.r.4.KR.1
fn read_c_2_r_4_kr_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let value = first_attr(
		xpath,
		node,
		CMfdsSafetyReportPaths::QUALIFICATION_KR1_CODE,
		"code",
	);
	import_constraint::string(
		"qualificationKr1",
		value.as_deref(),
		None,
		input_contracts::generated::c::mfds_c_2_r_4_kr_1,
	)?;
	Ok(value)
}

/// e2b:C.2.r.5
fn read_c_2_r_5(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let path = ".//hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code";
	let value = first_attr(xpath, node, path, "code")
		.map(|value| value.to_ascii_uppercase());
	let null_flavor = first_attr(xpath, node, path, "nullFlavor");
	import_constraint::string(
		"reporterCountry",
		value.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::c::c_2_r_3,
	)?;
	Ok(value)
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
		let (reporter_title, reporter_title_null_flavor) =
			read_c_2_r_1_1(&mut xpath, &node)?;
		let (reporter_given_name, reporter_given_name_null_flavor) =
			read_c_2_r_1_2(&mut xpath, &node)?;
		let (reporter_middle_name, reporter_middle_name_null_flavor) =
			read_c_2_r_1_3(&mut xpath, &node)?;
		let (reporter_family_name, reporter_family_name_null_flavor) =
			read_c_2_r_1_4(&mut xpath, &node)?;
		let (
			organization,
			organization_null_flavor,
			department,
			department_null_flavor,
		) = read_c_2_r_2_1_2(&mut xpath, &node)?;
		let (street, street_null_flavor) = read_c_2_r_2_3(&mut xpath, &node)?;
		let (city, city_null_flavor) = read_c_2_r_2_4(&mut xpath, &node)?;
		let (state, state_null_flavor) = read_c_2_r_2_5(&mut xpath, &node)?;
		let (postcode, postcode_null_flavor) = read_c_2_r_2_6(&mut xpath, &node)?;
		let (telephone, telephone_null_flavor) = read_c_2_r_2_7(&mut xpath, &node)?;
		let (email, email_null_flavor) = read_fda_c_2_r_2_8(&mut xpath, &node)?;
		let country_code = read_c_2_r_5(&mut xpath, &node)?;
		let (qualification, qualification_null_flavor) =
			read_c_2_r_4(&mut xpath, &node)?;
		let qualification_kr1 = read_c_2_r_4_kr_1(&mut xpath, &node)?;
		let primary_source_regulatory = read_c_2_r_3(&mut xpath, &node)?;

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
			email_null_flavor.as_ref(),
			qualification.as_ref(),
			qualification_null_flavor.as_ref(),
			qualification_kr1.as_ref(),
			primary_source_regulatory.as_ref(),
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
			email_null_flavor,
			qualification,
			qualification_null_flavor,
			qualification_kr1,
			primary_source_regulatory,
		});
	}

	Ok(items)
}

#[cfg(test)]
mod tests {
	use super::{
		parse_documents_held_by_sender, parse_literature_references,
		parse_primary_sources, parse_receiver_information, parse_sender_information,
		parse_study_information,
	};

	fn scenario6_xml() -> Vec<u8> {
		let root =
			std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
		std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario6.xml"))
			.expect("read scenario 6 fixture")
	}

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
	fn reporter_email_null_flavor_stays_separate_from_telephone() {
		let xml = primary_source_xml(
			r#"<telecom value="tel:+821012345678"/><telecom value="mailto" nullFlavor="NASK"/>"#,
		);
		let sources = parse_primary_sources(xml.as_bytes()).expect("parse");
		assert_eq!(sources[0].email_null_flavor.as_deref(), Some("NASK"));
		assert_eq!(sources[0].telephone_null_flavor, None);
	}

	#[test]
	fn sender_import_reads_mfds_qualification_from_assigned_person() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><subjectOf1><controlActEvent><author><assignedEntity><code code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.7"/><assignedPerson><asQualifiedEntity classCode="QUAL"><code code="4" codeSystem="2.16.840.1.113883.3.989.5.1.10.1.2"/></asQualifiedEntity></assignedPerson></assignedEntity></author></controlActEvent></subjectOf1></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let sender = parse_sender_information(xml)
			.expect("parse")
			.expect("sender");
		assert_eq!(sender.health_professional_type_kr1.as_deref(), Some("4"));
	}

	#[test]
	fn reporter_department_null_flavor_does_not_read_nested_organization() {
		let xml = primary_source_xml(
			r#"<representedOrganization><name nullFlavor="ASKU"/><assignedEntity><representedOrganization><name>Organization</name></representedOrganization></assignedEntity></representedOrganization>"#,
		);
		let sources = parse_primary_sources(xml.as_bytes()).expect("parse reporter");
		assert_eq!(sources[0].department, None);
		assert_eq!(sources[0].department_null_flavor.as_deref(), Some("ASKU"));
		assert_eq!(sources[0].organization.as_deref(), Some("Organization"));
		assert_eq!(sources[0].organization_null_flavor, None);
	}

	#[test]
	fn attachment_import_keeps_reference_file_names() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3">
  <reference><document><code code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.27"/><title>Held</title><text mediaType="application/pdf" representation="B64"><reference value="held.pdf"/>QUJD</text></document></reference>
  <reference><document><code code="2" codeSystem="2.16.840.1.113883.3.989.2.1.1.27"/><bibliographicDesignationText>Paper</bibliographicDesignationText><text mediaType="text/plain" representation="B64"><reference value="paper.txt"/>REVG</text></document></reference>
</MCCI_IN200100UV01>"#;

		let documents = parse_documents_held_by_sender(xml).expect("documents");
		let literature = parse_literature_references(xml).expect("literature");
		assert_eq!(documents[0].file_name.as_deref(), Some("held.pdf"));
		assert_eq!(documents[0].document_base64.as_deref(), Some("QUJD"));
		assert_eq!(literature[0].file_name.as_deref(), Some("paper.txt"));
		assert_eq!(literature[0].document_base64.as_deref(), Some("REVG"));
	}

	#[test]
	fn attachment_import_validates_normalized_base64_per_field() {
		for (code, value, valid, field) in [
			(
				"1",
				"QUJD",
				true,
				"documentsHeldBySender[].includedDocument",
			),
			(
				"1",
				"Q U\nJ\tD",
				true,
				"documentsHeldBySender[].includedDocument",
			),
			(
				"1",
				"%%%",
				false,
				"documentsHeldBySender[].includedDocument",
			),
			("1", "A", false, "documentsHeldBySender[].includedDocument"),
			("2", "REVG", true, "literatureReferences[].documentBase64"),
			(
				"2",
				"R E\nV\tG",
				true,
				"literatureReferences[].documentBase64",
			),
			("2", "%%%", false, "literatureReferences[].documentBase64"),
			("2", "A", false, "literatureReferences[].documentBase64"),
		] {
			let xml = format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><reference><document><code code="{code}" codeSystem="2.16.840.1.113883.3.989.2.1.1.27"/><text representation="B64">{value}</text></document></reference></MCCI_IN200100UV01>"#
			);
			let result = if code == "1" {
				parse_documents_held_by_sender(xml.as_bytes()).map(|_| ())
			} else {
				parse_literature_references(xml.as_bytes()).map(|_| ())
			};
			if valid {
				result.expect("valid Base64");
			} else {
				assert!(result.unwrap_err().to_string().contains(field));
			}
		}
	}

	#[test]
	fn primary_source_import_keeps_direct_department_without_promoting_it() {
		let xml = primary_source_xml(
			r#"<representedOrganization>
  <name>Direct Reporter Org</name>
</representedOrganization>"#,
		);

		let primary_sources = parse_primary_sources(xml.as_bytes()).expect("parse");

		assert_eq!(primary_sources.len(), 1);
		assert_eq!(primary_sources[0].organization, None);
		assert_eq!(
			primary_sources[0].department.as_deref(),
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

	#[test]
	fn primary_source_import_leaves_country_null_flavor_to_business_validation() {
		let xml = primary_source_xml(
			r#"<assignedPerson>
  <asQualifiedEntity><code nullFlavor="UNK"/></asQualifiedEntity>
  <asLocatedEntity><location><code nullFlavor="NASK"/></location></asLocatedEntity>
</assignedPerson>
<telecom nullFlavor="NASK"/>"#,
		);
		parse_primary_sources(xml.as_bytes())
			.expect("business validation runs later");
	}

	#[test]
	fn primary_source_import_keeps_qualification_null_flavor_without_default_code() {
		let xml = primary_source_xml(
			r#"<assignedPerson>
  <asQualifiedEntity><code nullFlavor="UNK"/></asQualifiedEntity>
</assignedPerson>"#,
		);

		let primary_sources = parse_primary_sources(xml.as_bytes()).expect("parse");

		assert_eq!(primary_sources.len(), 1);
		assert!(primary_sources[0].qualification.is_none());
		assert_eq!(
			primary_sources[0].qualification_null_flavor.as_deref(),
			Some("UNK")
		);
	}

	#[test]
	fn primary_source_import_preserves_mfds_qualification() {
		let xml = primary_source_xml(
			r#"<assignedPerson>
	<asQualifiedEntity><code code="1" codeSystem="2.16.840.1.113883.3.989.5.1.10.1.1"/></asQualifiedEntity>
	<asQualifiedEntity><code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.6"/></asQualifiedEntity>
</assignedPerson>"#,
		);

		let primary_sources = parse_primary_sources(xml.as_bytes()).expect("parse");

		assert_eq!(primary_sources.len(), 1);
		assert_eq!(primary_sources[0].qualification.as_deref(), Some("3"));
		assert_eq!(primary_sources[0].qualification_kr1.as_deref(), Some("1"));
	}

	#[test]
	fn c_sender_study_and_receiver_import_preserve_fixture_fields() {
		let xml = scenario6_xml();

		let sender = parse_sender_information(&xml)
			.expect("parse sender")
			.expect("sender should exist");
		assert_eq!(sender.sender_type.as_deref(), Some("1"));
		assert_eq!(sender.organization_name.as_deref(), Some("Big Pharma"));
		assert_eq!(sender.department.as_deref(), Some("Management"));
		assert_eq!(
			sender.street_address.as_deref(),
			Some("49 Main St. Building 2030A")
		);
		assert_eq!(sender.city.as_deref(), Some("Anytown"));
		assert_eq!(sender.state.as_deref(), Some("CT"));
		assert_eq!(sender.postcode.as_deref(), Some("23456"));
		assert_eq!(sender.country_code.as_deref(), Some("US"));
		assert_eq!(sender.person_title.as_deref(), Some("Mr"));
		assert_eq!(sender.person_given_name.as_deref(), Some("Charles"));
		assert_eq!(sender.person_middle_name.as_deref(), Some("Castile"));
		assert_eq!(sender.person_family_name.as_deref(), Some("Conner"));
		assert_eq!(sender.telephone.as_deref(), Some("8884562344"));
		assert_eq!(sender.fax.as_deref(), Some("6109991122"));
		assert_eq!(sender.email.as_deref(), Some("emailAddress@company.com"));

		let study = parse_study_information(&xml)
			.expect("parse study")
			.expect("study should exist");
		assert_eq!(study.study_name.as_deref(), Some("Profound Study"));
		assert_eq!(study.sponsor_study_number.as_deref(), Some("4555-3"));
		assert_eq!(study.registrations.len(), 2);
		assert_eq!(study.registrations[0].registration_number, "23489-34");
		assert_eq!(study.registrations[0].country_code.as_deref(), Some("US"));
		assert_eq!(study.registrations[1].registration_number, "876444");
		assert_eq!(study.registrations[1].country_code.as_deref(), Some("FR"));

		let receiver = parse_receiver_information(&xml).expect("parse receiver");
		assert_eq!(
			receiver.and_then(|receiver| receiver.organization_name),
			None,
			"receiver ID must not be imported as organization name"
		);
	}

	#[test]
	fn study_import_reads_mfds_type_from_investigation_event() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><component><adverseEventAssessment><subject1><primaryRole><subjectOf1><researchStudy classCode="CLNTRL" moodCode="EVN"><title>Study</title></researchStudy></subjectOf1></primaryRole></subject1></adverseEventAssessment></component><subjectOf2 typeCode="SUBJ"><investigationCharacteristic classCode="OBS" moodCode="EVN"><code code="1" codeSystem="2.16.840.1.113883.3.989.5.1.10.1.7"/><value xsi:type="CE" code="4"/></investigationCharacteristic></subjectOf2></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;

		let study = parse_study_information(xml)
			.expect("parse study")
			.expect("study");
		assert_eq!(study.study_type_reaction_kr1.as_deref(), Some("4"));
	}

	#[test]
	fn sender_import_preserves_name_null_flavors() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><subjectOf1><controlActEvent><author><assignedEntity><code code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.7"/><assignedPerson><name><given nullFlavor="MSK"/><family nullFlavor="NASK"/></name></assignedPerson></assignedEntity></author></controlActEvent></subjectOf1></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;

		let sender = parse_sender_information(xml)
			.expect("parse sender")
			.expect("sender");
		assert_eq!(sender.person_given_name_null_flavor.as_deref(), Some("MSK"));
		assert_eq!(
			sender.person_family_name_null_flavor.as_deref(),
			Some("NASK")
		);
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
		let source = read_c_1_9_1_r_1(&node)?;
		let extension = read_c_1_9_1_r_2(&node)?;
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

/// e2b:C.1.9.1.r.1
fn read_c_1_9_1_r_1(node: &libxml::tree::Node) -> Result<Option<String>> {
	let value = node.get_attribute("assigningAuthorityName");
	import_constraint::string(
		"otherCaseIdentifiers[].source",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_9_1_r_1,
	)?;
	Ok(value)
}

/// e2b:C.1.9.1.r.2
fn read_c_1_9_1_r_2(node: &libxml::tree::Node) -> Result<Option<String>> {
	let value = node.get_attribute("extension");
	import_constraint::string(
		"otherCaseIdentifiers[].caseIdentifier",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_9_1_r_2,
	)?;
	Ok(value)
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
		let extension = read_c_1_10_r(&node)?;
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

/// e2b:C.1.10.r
fn read_c_1_10_r(node: &libxml::tree::Node) -> Result<Option<String>> {
	let value = node.get_attribute("extension");
	import_constraint::string(
		"linkedReports[].linkedReportNumber",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_10_r,
	)?;
	Ok(value)
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
		let title = read_c_1_6_1_r_1(&mut xpath, &node)?;
		let (document_base64, file_name, media_type, representation, compression) =
			read_c_1_6_1_r_2(&mut xpath, &node)?;
		items.push(DocumentHeldImport {
			title,
			document_base64,
			file_name,
			media_type,
			representation,
			compression,
		});
	}
	Ok(items)
}

/// e2b:C.1.6.1.r.1
fn read_c_1_6_1_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let value = first_text(xpath, node, "hl7:title");
	import_constraint::string(
		"documentsHeldBySender[].documentDescription",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_1_6_1_r_1,
	)?;
	Ok(value)
}

/// e2b:C.1.6.1.r.2
/// e2b:C.1.6.1.r.2.local.mediaType
/// e2b:C.1.6.1.r.2.local.representation
/// e2b:C.1.6.1.r.2.local.compression
fn read_c_1_6_1_r_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
)> {
	let values = read_document_content(xpath, node)?;
	import_constraint::string(
		"documentsHeldBySender[].includedDocument",
		values.0.as_deref(),
		None,
		input_contracts::generated::c::c_1_6_1_r_2,
	)?;
	import_constraint::base64(
		"documentsHeldBySender[].includedDocument",
		values.0.as_deref(),
	)?;
	Ok(values)
}

fn read_document_content(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
)> {
	let representation = first_attr(xpath, node, "hl7:text", "representation");
	let document = xpath
		.findnodes("hl7:text", Some(node))
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query document text".to_string(),
			line: None,
			column: None,
		})?
		.into_iter()
		.next()
		.map(|text| {
			text.get_child_nodes()
				.into_iter()
				.filter(|child| child.get_type() == Some(NodeType::TextNode))
				.map(|child| child.get_content())
				.collect::<String>()
		})
		.filter(|value| !value.trim().is_empty())
		.map(|value| {
			if representation.as_deref() == Some("B64") {
				value.chars().filter(|c| !c.is_ascii_whitespace()).collect()
			} else {
				value
			}
		});
	Ok((
		document,
		first_attr(xpath, node, "hl7:text/hl7:reference", "value"),
		first_attr(xpath, node, "hl7:text", "mediaType"),
		representation,
		first_attr(xpath, node, "hl7:text", "compression"),
	))
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
		let (reference_text, reference_text_null_flavor) =
			read_c_4_r_1(&mut xpath, &node, idx)?;
		let (document_base64, file_name, media_type, representation, compression) =
			read_c_4_r_2(&mut xpath, &node)?;
		items.push(LiteratureImport {
			reference_text,
			reference_text_null_flavor,
			document_base64,
			file_name,
			media_type,
			representation,
			compression,
		});
	}
	Ok(items)
}

/// e2b:C.4.r.1
/// e2b:C.4.r.local.referenceTextNullFlavor
fn read_c_4_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
	_index: usize,
) -> Result<(Option<String>, Option<String>)> {
	let null_flavor = first_attr(
		xpath,
		node,
		"hl7:bibliographicDesignationText",
		"nullFlavor",
	)
	.or_else(|| first_attr(xpath, node, "hl7:title", "nullFlavor"));
	let text = first_text(xpath, node, "hl7:bibliographicDesignationText")
		.or_else(|| first_text(xpath, node, "hl7:title"));
	import_constraint::string(
		"referenceText",
		text.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::c::c_4_r_1,
	)?;
	import_constraint::string(
		"referenceTextNullFlavor",
		None,
		None,
		input_contracts::generated::c::c_4_r_1,
	)?;
	Ok((text, null_flavor))
}

/// e2b:C.4.r.2
/// e2b:C.4.r.2.local.mediaType
/// e2b:C.4.r.2.local.representation
/// e2b:C.4.r.2.local.compression
fn read_c_4_r_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
)> {
	let values = read_document_content(xpath, node)?;
	import_constraint::string(
		"literatureReferences[].documentBase64",
		values.0.as_deref(),
		None,
		input_contracts::generated::c::c_4_r_2,
	)?;
	import_constraint::base64(
		"literatureReferences[].documentBase64",
		values.0.as_deref(),
	)?;
	Ok(values)
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
	let Some(node) = nodes.first() else {
		return Ok(None);
	};

	let (study_name, study_name_null_flavor) = read_c_5_2(&mut xpath, node)?;
	let (sponsor_study_number, sponsor_study_number_null_flavor) =
		read_c_5_3(&mut xpath, node)?;
	let study_type_reaction = read_c_5_4(&mut xpath, node)?;
	let study_type_reaction_kr1 = read_c_5_4_kr_1(&mut xpath, node)?;
	let fda_ind_number_occurred = read_fda_c_5_5a(&mut xpath, node)?;
	let fda_pre_anda_number_occurred = read_fda_c_5_5b(&mut xpath, node)?;
	let cross_reported_inds = read_fda_c_5_6_r(&mut xpath, node)?;

	let reg_nodes = xpath
		.findnodes(".//hl7:studyRegistration", Some(node))
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query study registrations".to_string(),
			line: None,
			column: None,
		})?;
	let mut registrations = Vec::new();
	for reg in reg_nodes {
		let (registration_number, registration_number_null_flavor) =
			read_c_5_1_r_1(&mut xpath, &reg)?;
		let Some(registration_number) = registration_number.or_else(|| {
			registration_number_null_flavor
				.as_ref()
				.map(|_| String::new())
		}) else {
			continue;
		};
		let (country_code, country_code_null_flavor) =
			read_c_5_1_r_2(&mut xpath, &reg)?;
		registrations.push(StudyRegistrationImport {
			registration_number,
			registration_number_null_flavor,
			country_code,
			country_code_null_flavor,
		});
	}

	Ok(Some(StudyImport {
		study_name,
		study_name_null_flavor,
		sponsor_study_number,
		sponsor_study_number_null_flavor,
		study_type_reaction,
		study_type_reaction_kr1,
		fda_ind_number_occurred,
		fda_pre_anda_number_occurred,
		registrations,
		cross_reported_inds,
	}))
}

/// e2b:C.5.1.r.1
fn read_c_5_1_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	input_pair(
		"studyRegistrationNumbers[].registrationNumber",
		first_attr(xpath, node, "hl7:id", "extension"),
		first_attr(xpath, node, "hl7:id", "nullFlavor"),
		input_contracts::generated::c::c_5_1_r_1,
	)
}

/// e2b:C.5.1.r.2
fn read_c_5_1_r_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let path = "hl7:author/hl7:territorialAuthority/hl7:governingPlace/hl7:code";
	let value = first_attr(xpath, node, path, "code")
		.map(|value| value.to_ascii_uppercase());
	input_pair(
		"studyRegistrationNumbers[].countryCode",
		value,
		first_attr(xpath, node, path, "nullFlavor"),
		input_contracts::generated::c::c_5_1_r_2,
	)
}

/// e2b:C.5.2
fn read_c_5_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let (value, null_flavor) = read_text_with_null_flavor(xpath, node, "hl7:title");
	input_pair(
		"studyName",
		value,
		null_flavor,
		input_contracts::generated::c::c_5_2,
	)
}

/// e2b:C.5.3
fn read_c_5_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	input_pair(
		"sponsorStudyNumber",
		first_attr(xpath, node, "hl7:id", "extension"),
		first_attr(xpath, node, "hl7:id", "nullFlavor"),
		input_contracts::generated::c::c_5_3,
	)
}

/// e2b:C.5.4
fn read_c_5_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let value = first_attr(xpath, node, "hl7:code", "code");
	import_constraint::string(
		"studyTypeReaction",
		value.as_deref(),
		None,
		input_contracts::generated::c::c_5_4,
	)?;
	Ok(value)
}

/// e2b:C.5.4.KR.1
fn read_c_5_4_kr_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let value = first_attr(
		xpath,
		node,
		"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.5.1.10.1.7']]/hl7:value",
		"code",
	);
	import_constraint::string(
		"studyTypeReactionKr1",
		value.as_deref(),
		None,
		input_contracts::generated::c::mfds_c_5_4_kr_1,
	)?;
	Ok(value)
}

/// e2b:FDA.C.5.5a
fn read_fda_c_5_5a(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let value = first_attr(
		xpath,
		node,
		"hl7:id[@root='2.16.840.1.113883.3.989.5.1.2.2.1.2.1']",
		"extension",
	);
	import_constraint::string(
		"fdaIndNumberOccurred",
		value.as_deref(),
		None,
		input_contracts::generated::c::fda_c_5_5a,
	)?;
	Ok(value)
}

/// e2b:FDA.C.5.5b
fn read_fda_c_5_5b(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let value = first_attr(
		xpath,
		node,
		"hl7:id[@root='2.16.840.1.113883.3.989.5.1.2.2.1.2.2']",
		"extension",
	);
	import_constraint::string(
		"fdaPreAndaNumberOccurred",
		value.as_deref(),
		None,
		input_contracts::generated::c::fda_c_5_5b,
	)?;
	Ok(value)
}

/// e2b:FDA.C.5.6.r
/// e2b:FDA.C.5.6.r.local.indNumberNullFlavor
fn read_fda_c_5_6_r(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Vec<(Option<String>, Option<String>)>> {
	xpath
		.findnodes(
			"hl7:id[@root='2.16.840.1.113883.3.989.5.1.2.2.1.2.3']",
			Some(node),
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query FDA cross-reported identifiers".to_string(),
			line: None,
			column: None,
		})?
		.into_iter()
		.map(|id| {
			input_pair(
				"fdaCrossReportedIndNumbers[].indNumber",
				id.get_property("extension"),
				id.get_property("nullFlavor"),
				input_contracts::generated::c::fda_c_5_6_r,
			)
		})
		.collect()
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
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query receiver information".to_string(),
			line: None,
			column: None,
		})?
		.into_iter()
		.next();

	let organization_name = read_receiver_organization_name(&mut xpath);

	if organization_name.is_none() {
		return Ok(None);
	}

	Ok(Some(ReceiverInformationForUpdate {
		receiver_type: normalize_code(
			read_receiver_type(&mut xpath),
			&["1", "2", "3", "4", "5", "6"],
			"receiver_information.receiver_type",
		),
		organization_name,
		department: read_receiver_department(&mut xpath),
		street_address: read_receiver_street_address(&mut xpath),
		city: read_receiver_city(&mut xpath),
		state_province: read_receiver_state_province(&mut xpath),
		postcode: read_receiver_postcode(&mut xpath),
		country_code: normalize_iso2(
			read_receiver_country(&mut xpath),
			"receiver_information.country_code",
		),
		telephone: read_receiver_telephone(&mut xpath, receiver_node.as_ref()),
		fax: read_receiver_fax(&mut xpath, receiver_node.as_ref()),
		email: read_receiver_email(&mut xpath, receiver_node.as_ref()),
	}))
}

/// e2b:local.receiver.1
fn read_receiver_type(xpath: &mut Context) -> Option<String> {
	first_value_root(xpath, "//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:code/@code")
}

/// e2b:local.receiver.2
fn read_receiver_organization_name(xpath: &mut Context) -> Option<String> {
	first_text_root(
		xpath,
		"//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:name",
	)
}

/// e2b:local.receiver.3
fn read_receiver_department(xpath: &mut Context) -> Option<String> {
	first_text_root(
		xpath,
		"//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:desc",
	)
}

/// e2b:local.receiver.4
fn read_receiver_street_address(xpath: &mut Context) -> Option<String> {
	first_text_root(
		xpath,
		"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:streetAddressLine",
	)
}

/// e2b:local.receiver.5
fn read_receiver_city(xpath: &mut Context) -> Option<String> {
	first_text_root(
		xpath,
		"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:city",
	)
}

/// e2b:local.receiver.6
fn read_receiver_state_province(xpath: &mut Context) -> Option<String> {
	first_text_root(
		xpath,
		"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:state",
	)
}

/// e2b:local.receiver.7
fn read_receiver_postcode(xpath: &mut Context) -> Option<String> {
	first_text_root(
		xpath,
		"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:postalCode",
	)
}

/// e2b:local.receiver.country
fn read_receiver_country(xpath: &mut Context) -> Option<String> {
	first_value_root(
		xpath,
		"//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:country/@code",
	)
}

/// e2b:local.receiver.8
fn read_receiver_telephone(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Option<String> {
	node.and_then(|node| telecom_first_in_node(xpath, node, "tel:"))
}

/// e2b:local.receiver.9
fn read_receiver_fax(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Option<String> {
	node.and_then(|node| telecom_first_in_node(xpath, node, "fax:"))
}

/// e2b:local.receiver.10
fn read_receiver_email(
	xpath: &mut Context,
	node: Option<&libxml::tree::Node>,
) -> Option<String> {
	node.and_then(|node| telecom_first_in_node(xpath, node, "mailto:"))
}
