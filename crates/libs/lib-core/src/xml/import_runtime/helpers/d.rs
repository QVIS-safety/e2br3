use crate::xml::error::Error;
use crate::xml::import_runtime::shared::{
	clamp_str, first_attr, first_text, first_value, first_value_root,
	normalize_code, normalize_code3, normalize_sex_code, parse_bool_attr,
	parse_bool_value, parse_date,
};
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use sqlx::types::time::Date;

pub(crate) struct PatientImport {
	pub(crate) patient_initials: Option<String>,
	pub(crate) patient_given_name: Option<String>,
	pub(crate) patient_family_name: Option<String>,
	pub(crate) patient_initials_null_flavor: Option<String>,
	pub(crate) birth_date: Option<Date>,
	pub(crate) birth_date_null_flavor: Option<String>,
	pub(crate) sex: Option<String>,
	pub(crate) sex_null_flavor: Option<String>,
	pub(crate) age_at_time_of_onset: Option<Decimal>,
	pub(crate) age_at_time_of_onset_null_flavor: Option<String>,
	pub(crate) age_unit: Option<String>,
	pub(crate) gestation_period: Option<Decimal>,
	pub(crate) gestation_period_unit: Option<String>,
	pub(crate) age_group: Option<String>,
	pub(crate) weight_kg: Option<Decimal>,
	pub(crate) height_cm: Option<Decimal>,
	pub(crate) race_code: Option<String>,
	pub(crate) race_code_null_flavor: Option<String>,
	pub(crate) ethnicity_code: Option<String>,
	pub(crate) ethnicity_code_null_flavor: Option<String>,
	pub(crate) last_menstrual_period_date: Option<Date>,
	pub(crate) last_menstrual_period_date_null_flavor: Option<String>,
	pub(crate) medical_history_text: Option<String>,
	pub(crate) concomitant_therapy: Option<bool>,
}

#[derive(Debug)]
pub(crate) struct PatientIdentifierImport {
	pub(crate) identifier_type_code: String,
	pub(crate) identifier_value: String,
}

#[derive(Debug)]
pub(crate) struct MedicalHistoryImport {
	pub(crate) meddra_version: Option<String>,
	pub(crate) meddra_code: Option<String>,
	pub(crate) start_date: Option<Date>,
	pub(crate) continuing: Option<bool>,
	pub(crate) end_date: Option<Date>,
	pub(crate) comments: Option<String>,
	pub(crate) family_history: Option<bool>,
}

#[derive(Debug)]
pub(crate) struct PastDrugHistoryImport {
	pub(crate) drug_name: Option<String>,
	pub(crate) mpid: Option<String>,
	pub(crate) mpid_version: Option<String>,
	pub(crate) mfds_medicinal_product_version: Option<String>,
	pub(crate) mfds_medicinal_product_id: Option<String>,
	pub(crate) phpid: Option<String>,
	pub(crate) phpid_version: Option<String>,
	pub(crate) start_date: Option<Date>,
	pub(crate) end_date: Option<Date>,
	pub(crate) indication_meddra_version: Option<String>,
	pub(crate) indication_meddra_code: Option<String>,
	pub(crate) reaction_meddra_version: Option<String>,
	pub(crate) reaction_meddra_code: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DeathImport {
	pub(crate) date_of_death: Option<Date>,
	pub(crate) date_of_death_null_flavor: Option<String>,
	pub(crate) autopsy_performed: Option<bool>,
	pub(crate) reported_causes: Vec<DeathCauseImport>,
	pub(crate) autopsy_causes: Vec<DeathCauseImport>,
}

#[derive(Debug)]
pub(crate) struct DeathCauseImport {
	pub(crate) meddra_version: Option<String>,
	pub(crate) meddra_code: Option<String>,
	pub(crate) comments: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ParentImport {
	pub(crate) parent_identification: Option<String>,
	pub(crate) parent_birth_date: Option<Date>,
	pub(crate) parent_birth_date_null_flavor: Option<String>,
	pub(crate) parent_age: Option<Decimal>,
	pub(crate) parent_age_null_flavor: Option<String>,
	pub(crate) parent_age_unit: Option<String>,
	pub(crate) last_menstrual_period_date: Option<Date>,
	pub(crate) last_menstrual_period_date_null_flavor: Option<String>,
	pub(crate) weight_kg: Option<Decimal>,
	pub(crate) height_cm: Option<Decimal>,
	pub(crate) sex: Option<String>,
	pub(crate) medical_history_text: Option<String>,
	pub(crate) medical_history: Vec<MedicalHistoryImport>,
	pub(crate) past_drugs: Vec<PastDrugHistoryImport>,
}

pub(crate) fn parse_patient_identifiers(
	xml: &[u8],
) -> Result<Vec<PatientIdentifierImport>> {
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
		.findnodes("//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity", None)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query patient identifiers".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let identifier_type_code = normalize_code(
			first_attr(&mut xpath, &node, "hl7:code", "code"),
			&["1", "2", "3", "4"],
			"patient_identifiers.identifier_type_code",
		);
		let identifier_value = first_attr(&mut xpath, &node, "hl7:id", "extension");
		if let (Some(identifier_type_code), Some(identifier_value)) =
			(identifier_type_code, identifier_value)
		{
			items.push(PatientIdentifierImport {
				identifier_type_code,
				identifier_value,
			});
		}
	}
	Ok(items)
}

pub(crate) fn parse_medical_history(
	xml: &[u8],
) -> Result<Vec<MedicalHistoryImport>> {
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
			"//hl7:organizer[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:observation",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query medical history".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let code_system = first_attr(&mut xpath, &node, "hl7:code", "codeSystem");
		if code_system.as_deref() != Some("2.16.840.1.113883.6.163") {
			continue;
		}
		let meddra_code = first_attr(&mut xpath, &node, "hl7:code", "code");
		let meddra_version = clamp_str(
			first_attr(&mut xpath, &node, "hl7:code", "codeSystemVersion"),
			10,
			"medical_history.meddra_version",
		);
		let start_date =
			first_attr(&mut xpath, &node, "hl7:effectiveTime/hl7:low", "value")
				.and_then(parse_date);
		let end_date =
			first_attr(&mut xpath, &node, "hl7:effectiveTime/hl7:high", "value")
				.and_then(parse_date);
		let continuing = parse_bool_attr(
			&mut xpath,
			&node,
			"hl7:inboundRelationship/hl7:observation[hl7:code[@code='13']]/hl7:value",
			"value",
		);
		let comments = first_text(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='10']]/hl7:value",
		);
		let family_history = parse_bool_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='38']]/hl7:value",
			"value",
		);
		items.push(MedicalHistoryImport {
			meddra_version,
			meddra_code,
			start_date,
			continuing,
			end_date,
			comments,
			family_history,
		});
	}
	Ok(items)
}

pub(crate) fn parse_past_drug_history(
	xml: &[u8],
) -> Result<Vec<PastDrugHistoryImport>> {
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
			"//hl7:organizer[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query past drug history".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let drug_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name",
		);
		let mfds_medicinal_product_id = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code",
				"code",
			),
			10,
			"past_drug_history.mfds_medicinal_product_id",
		);
		let mfds_medicinal_product_version = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code",
				"codeSystemVersion",
			),
			20,
			"past_drug_history.mfds_medicinal_product_version",
		);
		let mpid = first_value(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension",
		);
		let mpid_version = clamp_str(
			first_value(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion",
			),
			10,
			"past_drug_history.mpid_version",
		);
		let phpid = first_value(
			&mut xpath,
			&node,
			"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:id/@extension)[1]",
		);
		let phpid_version = clamp_str(
			first_value(
				&mut xpath,
				&node,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:code/@codeSystemVersion)[1]",
			),
			10,
			"past_drug_history.phpid_version",
		);
		let start_date =
			first_attr(&mut xpath, &node, "hl7:effectiveTime/hl7:low", "value")
				.and_then(parse_date);
		let end_date =
			first_attr(&mut xpath, &node, "hl7:effectiveTime/hl7:high", "value")
				.and_then(parse_date);
		let indication_meddra_code = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value",
			"code",
		);
		let indication_meddra_version = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value",
				"codeSystemVersion",
			),
			10,
			"past_drug_history.indication_meddra_version",
		);
		let reaction_meddra_code = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value",
			"code",
		);
		let reaction_meddra_version = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value",
				"codeSystemVersion",
			),
			10,
			"past_drug_history.reaction_meddra_version",
		);
		items.push(PastDrugHistoryImport {
			drug_name,
			mpid,
			mpid_version,
			mfds_medicinal_product_version,
			mfds_medicinal_product_id,
			phpid,
			phpid_version,
			start_date,
			end_date,
			indication_meddra_version,
			indication_meddra_code,
			reaction_meddra_version,
			reaction_meddra_code,
		});
	}
	Ok(items)
}

pub(crate) fn parse_patient_death(xml: &[u8]) -> Result<Option<DeathImport>> {
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

	let date_of_death = first_value_root(&mut xpath, "//hl7:deceasedTime/@value")
		.and_then(parse_date);
	let date_of_death_null_flavor =
		first_value_root(&mut xpath, "//hl7:deceasedTime/@nullFlavor");
	let autopsy_performed = parse_bool_value(first_value_root(
		&mut xpath,
		"//hl7:observation[hl7:code[@code='5']]/hl7:value/@value",
	));

	let mut reported_causes = Vec::new();
	let reported_nodes = xpath
		.findnodes("//hl7:observation[hl7:code[@code='32']]/hl7:value", None)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query reported causes of death".to_string(),
			line: None,
			column: None,
		})?;
	for node in reported_nodes {
		let meddra_code = node.get_attribute("code");
		let meddra_version = clamp_str(
			node.get_attribute("codeSystemVersion"),
			10,
			"death.meddra_version",
		);
		let comments = first_text(&mut xpath, &node, "hl7:originalText");
		reported_causes.push(DeathCauseImport {
			meddra_version,
			meddra_code,
			comments,
		});
	}

	let mut autopsy_causes = Vec::new();
	let autopsy_nodes = xpath
		.findnodes(
			"//hl7:observation[hl7:code[@code='5']]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='8']]/hl7:value",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query autopsy causes of death".to_string(),
			line: None,
			column: None,
		})?;
	for node in autopsy_nodes {
		let meddra_code = node.get_attribute("code");
		let meddra_version = clamp_str(
			node.get_attribute("codeSystemVersion"),
			10,
			"death.autopsy_meddra_version",
		);
		let comments = first_text(&mut xpath, &node, "hl7:originalText");
		autopsy_causes.push(DeathCauseImport {
			meddra_version,
			meddra_code,
			comments,
		});
	}

	if date_of_death.is_none()
		&& date_of_death_null_flavor.is_none()
		&& autopsy_performed.is_none()
		&& reported_causes.is_empty()
		&& autopsy_causes.is_empty()
	{
		return Ok(None);
	}

	Ok(Some(DeathImport {
		date_of_death,
		date_of_death_null_flavor,
		autopsy_performed,
		reported_causes,
		autopsy_causes,
	}))
}

pub(crate) fn parse_parent_information(xml: &[u8]) -> Result<Option<ParentImport>> {
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
			"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query parent information".to_string(),
			line: None,
			column: None,
		})?;
	let Some(node) = nodes.get(0) else {
		return Ok(None);
	};

	let parent_identification =
		first_text(&mut xpath, node, "hl7:associatedPerson/hl7:name");
	let parent_birth_date = first_attr(
		&mut xpath,
		node,
		"hl7:associatedPerson/hl7:birthTime",
		"value",
	)
	.and_then(parse_date);
	let parent_birth_date_null_flavor = first_attr(
		&mut xpath,
		node,
		"hl7:associatedPerson/hl7:birthTime",
		"nullFlavor",
	);
	let sex = normalize_sex_code(first_attr(
		&mut xpath,
		node,
		"hl7:associatedPerson/hl7:administrativeGenderCode",
		"code",
	));
	let parent_age = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let parent_age_null_flavor = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
		"nullFlavor",
	);
	let parent_age_unit = normalize_code3(
		first_attr(
			&mut xpath,
			node,
			"hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
			"unit",
		),
		"parent_information.parent_age_unit",
	);
	let last_menstrual_period_date = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
		"value",
	)
	.and_then(parse_date);
	let last_menstrual_period_date_null_flavor = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
		"nullFlavor",
	);
	let weight_kg = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='7']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let height_cm = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='17']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let medical_history_text = first_text(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
	);

	let mut medical_history = Vec::new();
	let history_nodes = xpath
		.findnodes(
			"hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation",
			Some(node),
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query parent medical history".to_string(),
			line: None,
			column: None,
		})?;
	for obs in history_nodes {
		let code_system = first_attr(&mut xpath, &obs, "hl7:code", "codeSystem");
		if code_system.as_deref() != Some("2.16.840.1.113883.6.163") {
			continue;
		}
		let meddra_code = first_attr(&mut xpath, &obs, "hl7:code", "code");
		let meddra_version = clamp_str(
			first_attr(&mut xpath, &obs, "hl7:code", "codeSystemVersion"),
			10,
			"parent_history.meddra_version",
		);
		let start_date =
			first_attr(&mut xpath, &obs, "hl7:effectiveTime/hl7:low", "value")
				.and_then(parse_date);
		let end_date =
			first_attr(&mut xpath, &obs, "hl7:effectiveTime/hl7:high", "value")
				.and_then(parse_date);
		let continuing = parse_bool_attr(
			&mut xpath,
			&obs,
			"hl7:inboundRelationship/hl7:observation[hl7:code[@code='13']]/hl7:value",
			"value",
		);
		let comments = first_text(
			&mut xpath,
			&obs,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='10']]/hl7:value",
		);
		let family_history = parse_bool_attr(
			&mut xpath,
			&obs,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='38']]/hl7:value",
			"value",
		);
		medical_history.push(MedicalHistoryImport {
			meddra_version,
			meddra_code,
			start_date,
			continuing,
			end_date,
			comments,
			family_history,
		});
	}

	let mut past_drugs = Vec::new();
	let drug_nodes = xpath
		.findnodes(
			"hl7:subjectOf2/hl7:organizer[hl7:code[@code='2']]/hl7:component/hl7:substanceAdministration",
			Some(node),
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query parent past drugs".to_string(),
			line: None,
			column: None,
		})?;
	for obs in drug_nodes {
		let drug_name = first_text(
			&mut xpath,
			&obs,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name",
		);
		let mpid = first_value(
			&mut xpath,
			&obs,
			"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension)[1]",
		);
		let mpid_version = clamp_str(
			first_value(
				&mut xpath,
				&obs,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion)[1]",
			),
			10,
			"parent_past_drug.mpid_version",
		);
		let mfds_medicinal_product_version = clamp_str(
			first_value(
				&mut xpath,
				&obs,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@codeSystemVersion)[1]",
			),
			20,
			"parent_past_drug.mfds_medicinal_product_version",
		);
		let mfds_medicinal_product_id = clamp_str(
			first_value(
				&mut xpath,
				&obs,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@code)[1]",
			),
			10,
			"parent_past_drug.mfds_medicinal_product_id",
		);
		let start_date =
			first_attr(&mut xpath, &obs, "hl7:effectiveTime/hl7:low", "value")
				.and_then(parse_date);
		let end_date =
			first_attr(&mut xpath, &obs, "hl7:effectiveTime/hl7:high", "value")
				.and_then(parse_date);
		let indication_meddra_code = first_attr(
			&mut xpath,
			&obs,
			"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value",
			"code",
		);
		let indication_meddra_version = clamp_str(
			first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value",
				"codeSystemVersion",
			),
			10,
			"parent_past_drug.indication_meddra_version",
		);
		let reaction_meddra_code = first_attr(
			&mut xpath,
			&obs,
			"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value",
			"code",
		);
		let reaction_meddra_version = clamp_str(
			first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value",
				"codeSystemVersion",
			),
			10,
			"parent_past_drug.reaction_meddra_version",
		);
		past_drugs.push(PastDrugHistoryImport {
			drug_name,
			mpid,
			mpid_version,
			mfds_medicinal_product_version,
			mfds_medicinal_product_id,
			phpid: first_value(
				&mut xpath,
				&obs,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:id/@extension)[1]",
			),
			phpid_version: clamp_str(
				first_value(
					&mut xpath,
					&obs,
					"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:code/@codeSystemVersion)[1]",
				),
				10,
				"parent_past_drug.phpid_version",
			),
			start_date,
			end_date,
			indication_meddra_version,
			indication_meddra_code,
			reaction_meddra_version,
			reaction_meddra_code,
		});
	}

	Ok(Some(ParentImport {
		parent_identification,
		parent_birth_date,
		parent_birth_date_null_flavor,
		parent_age,
		parent_age_null_flavor,
		parent_age_unit,
		last_menstrual_period_date,
		last_menstrual_period_date_null_flavor,
		weight_kg,
		height_cm,
		sex,
		medical_history_text,
		medical_history,
		past_drugs,
	}))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_past_drug_uses_mfds_fields_separate_from_mpid() {
		let xml = br#"
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess>
      <subject>
        <investigationEvent>
          <subjectOf2>
            <primaryRole>
              <subjectOf2>
                <organizer>
                  <code code="2" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/>
                  <component>
                    <substanceAdministration>
                      <consumable>
                        <instanceOfKind>
                          <kindOfProduct>
                            <code code="KR-DH-ID" codeSystemVersion="KR-DH-V1"/>
                            <name>Past DH Drug</name>
                            <asIdentifiedEntity>
                              <id extension="MPID-EXACT"/>
                              <code code="MPID" codeSystemVersion="MPID-V1"/>
                            </asIdentifiedEntity>
                            <asIdentifiedEntity>
                              <id extension="PHPID-EXACT"/>
                              <code code="PHPID" codeSystemVersion="PHPID-V1"/>
                            </asIdentifiedEntity>
                          </kindOfProduct>
                        </instanceOfKind>
                      </consumable>
                    </substanceAdministration>
                  </component>
                </organizer>
              </subjectOf2>
            </primaryRole>
          </subjectOf2>
        </investigationEvent>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>
"#;

		let past_drugs = parse_past_drug_history(xml).expect("parse");
		let past_drug = past_drugs.first().expect("past drug");

		assert_eq!(
			past_drug.mfds_medicinal_product_version.as_deref(),
			Some("KR-DH-V1")
		);
		assert_eq!(
			past_drug.mfds_medicinal_product_id.as_deref(),
			Some("KR-DH-ID")
		);
		assert_eq!(past_drug.mpid.as_deref(), Some("MPID-EXACT"));
		assert_eq!(past_drug.mpid_version.as_deref(), Some("MPID-V1"));
		assert_eq!(past_drug.phpid.as_deref(), Some("PHPID-EXACT"));
		assert_eq!(past_drug.phpid_version.as_deref(), Some("PHPID-V1"));
	}

	#[test]
	fn parse_parent_past_drug_uses_mfds_fields_separate_from_mpid() {
		let xml = br#"
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess>
      <subject>
        <investigationEvent>
          <subjectOf2>
            <primaryRole>
              <player1>
                <role>
                  <code code="PRN"/>
                  <associatedPerson/>
                  <subjectOf2>
                    <organizer>
                      <code code="2"/>
                      <component>
                        <substanceAdministration>
                          <consumable>
                            <instanceOfKind>
                              <kindOfProduct>
                                <code code="MFDS-ID" codeSystemVersion="MFDS-V1"/>
                                <name>Parent MFDS Drug</name>
                                <asIdentifiedEntity>
                                  <id extension="MPID-EXACT"/>
                                  <code code="MPID" codeSystemVersion="MPID-V1"/>
                                </asIdentifiedEntity>
                                <asIdentifiedEntity>
                                  <id extension="PHPID-EXACT"/>
                                  <code code="PHPID" codeSystemVersion="PHPID-V1"/>
                                </asIdentifiedEntity>
                              </kindOfProduct>
                            </instanceOfKind>
                          </consumable>
                        </substanceAdministration>
                      </component>
                    </organizer>
                  </subjectOf2>
                </role>
              </player1>
            </primaryRole>
          </subjectOf2>
        </investigationEvent>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>
"#;

		let parent = parse_parent_information(xml)
			.expect("parse")
			.expect("parent should exist");
		let past_drug = parent.past_drugs.first().expect("parent past drug");

		assert_eq!(
			past_drug.mfds_medicinal_product_version.as_deref(),
			Some("MFDS-V1")
		);
		assert_eq!(
			past_drug.mfds_medicinal_product_id.as_deref(),
			Some("MFDS-ID")
		);
		assert_eq!(past_drug.mpid.as_deref(), Some("MPID-EXACT"));
		assert_eq!(past_drug.mpid_version.as_deref(), Some("MPID-V1"));
		assert_eq!(past_drug.phpid.as_deref(), Some("PHPID-EXACT"));
		assert_eq!(past_drug.phpid_version.as_deref(), Some("PHPID-V1"));
	}
}
