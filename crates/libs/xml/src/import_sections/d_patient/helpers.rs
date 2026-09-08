use crate::error::Error;
use crate::import_constraint;
use crate::import_sections::shared::{
	first_attr, first_text, first_value, first_value_root, normalize_code,
	parse_bool_attr, parse_bool_with_yes_no, parse_date,
};
use crate::Result;
use libxml::parser::Parser;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use sqlx::types::time::Date;

pub(crate) struct PatientImport {
	pub(crate) patient_initials: Option<String>,
	pub(crate) patient_initials_null_flavor: Option<String>,
	pub(crate) birth_date: Option<Date>,
	pub(crate) birth_date_null_flavor: Option<String>,
	pub(crate) sex: Option<String>,
	pub(crate) sex_null_flavor: Option<String>,
	pub(crate) age_at_time_of_onset: Option<Decimal>,
	pub(crate) age_unit: Option<String>,
	pub(crate) gestation_period: Option<Decimal>,
	pub(crate) gestation_period_unit: Option<String>,
	pub(crate) age_group: Option<String>,
	pub(crate) weight_kg: Option<Decimal>,
	pub(crate) height_cm: Option<Decimal>,
	pub(crate) race_codes: Vec<String>,
	pub(crate) race_code_null_flavor: Option<String>,
	pub(crate) ethnicity_code: Option<String>,
	pub(crate) ethnicity_code_null_flavor: Option<String>,
	pub(crate) last_menstrual_period_date: Option<Date>,
	pub(crate) last_menstrual_period_date_null_flavor: Option<String>,
	pub(crate) medical_history_text: Option<String>,
	pub(crate) medical_history_text_null_flavor: Option<String>,
	pub(crate) concomitant_therapy: Option<bool>,
}

#[derive(Debug)]
pub(crate) struct PatientIdentifierImport {
	pub(crate) identifier_type_code: String,
	pub(crate) identifier_value: Option<String>,
	pub(crate) identifier_value_null_flavor: Option<String>,
}

#[derive(Debug)]
pub(crate) struct MedicalHistoryImport {
	pub(crate) meddra_version: Option<String>,
	pub(crate) meddra_code: Option<String>,
	pub(crate) start_date: Option<String>,
	pub(crate) start_date_null_flavor: Option<String>,
	pub(crate) continuing: Option<bool>,
	pub(crate) continuing_null_flavor: Option<String>,
	pub(crate) end_date: Option<String>,
	pub(crate) end_date_null_flavor: Option<String>,
	pub(crate) comments: Option<String>,
	pub(crate) family_history: Option<bool>,
}

#[derive(Debug)]
pub(crate) struct ParentMedicalHistoryImport {
	pub(crate) meddra_version: Option<String>,
	pub(crate) meddra_code: Option<String>,
	pub(crate) start_date: Option<Date>,
	pub(crate) start_date_null_flavor: Option<String>,
	pub(crate) continuing: Option<bool>,
	pub(crate) continuing_null_flavor: Option<String>,
	pub(crate) end_date: Option<Date>,
	pub(crate) end_date_null_flavor: Option<String>,
	pub(crate) comments: Option<String>,
}

#[derive(Debug)]
pub(crate) struct PastDrugHistoryImport {
	pub(crate) drug_name: Option<String>,
	pub(crate) drug_name_null_flavor: Option<String>,
	pub(crate) mpid: Option<String>,
	pub(crate) mpid_version: Option<String>,
	pub(crate) mpid_source_code_system: Option<String>,
	pub(crate) mpid_source_code_system_version: Option<String>,
	pub(crate) mfds_medicinal_product_version: Option<String>,
	pub(crate) mfds_medicinal_product_id: Option<String>,
	pub(crate) phpid: Option<String>,
	pub(crate) phpid_version: Option<String>,
	pub(crate) start_date: Option<Date>,
	pub(crate) start_date_null_flavor: Option<String>,
	pub(crate) end_date: Option<Date>,
	pub(crate) end_date_null_flavor: Option<String>,
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
	pub(crate) autopsy_performed_null_flavor: Option<String>,
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
	pub(crate) parent_identification_null_flavor: Option<String>,
	pub(crate) parent_birth_date: Option<Date>,
	pub(crate) parent_birth_date_null_flavor: Option<String>,
	pub(crate) parent_age: Option<Decimal>,
	pub(crate) parent_age_unit: Option<String>,
	pub(crate) last_menstrual_period_date: Option<Date>,
	pub(crate) last_menstrual_period_date_null_flavor: Option<String>,
	pub(crate) weight_kg: Option<Decimal>,
	pub(crate) height_cm: Option<Decimal>,
	pub(crate) sex: Option<String>,
	pub(crate) sex_null_flavor: Option<String>,
	pub(crate) medical_history_text: Option<String>,
	pub(crate) medical_history: Vec<ParentMedicalHistoryImport>,
	pub(crate) past_drugs: Vec<PastDrugHistoryImport>,
}

fn input_string(
	value: Option<String>,
	field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<Option<String>> {
	import_constraint::string(field, value.as_deref(), None, check)?;
	Ok(value)
}

fn input_date(
	value: Option<String>,
	null_flavor: Option<String>,
	field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<(Option<Date>, Option<String>)> {
	import_constraint::string(
		field,
		value.as_deref(),
		null_flavor.as_deref(),
		check,
	)?;
	Ok((value.and_then(parse_date), null_flavor))
}

fn input_ts(
	value: Option<String>,
	null_flavor: Option<String>,
	field: &str,
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

fn input_number(
	value: Option<String>,
	field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<Option<Decimal>> {
	import_constraint::number_string(field, value.as_deref(), check)?;
	Ok(value.and_then(|value| value.parse().ok()))
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
		let identifier_type_code =
			read_d_local_patient_identifier_type_code(&mut xpath, &node);
		let (identifier_value, identifier_value_null_flavor) =
			read_d_local_patient_identifier_value(
				&mut xpath,
				&node,
				identifier_type_code.as_deref(),
			)?;
		if let (Some(identifier_type_code), Some(identifier_value)) =
			(identifier_type_code.clone(), identifier_value)
		{
			items.push(PatientIdentifierImport {
				identifier_type_code,
				identifier_value: Some(identifier_value),
				identifier_value_null_flavor: None,
			});
		} else if let (
			Some(identifier_type_code),
			Some(identifier_value_null_flavor),
		) = (identifier_type_code, identifier_value_null_flavor)
		{
			items.push(PatientIdentifierImport {
				identifier_type_code,
				identifier_value: None,
				identifier_value_null_flavor: Some(identifier_value_null_flavor),
			});
		}
	}
	Ok(items)
}

/// e2b:D.local.patientIdentifier.typeCode
fn read_d_local_patient_identifier_type_code(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	normalize_code(
		first_attr(xpath, node, "hl7:code", "code"),
		&["1", "2", "3", "4"],
		"patient_identifiers.identifier_type_code",
	)
}

/// e2b:D.local.patientIdentifier.value
/// e2b:D.1.1.1
/// e2b:D.1.1.2
/// e2b:D.1.1.3
/// e2b:D.1.1.4
fn read_d_local_patient_identifier_value(
	xpath: &mut Context,
	node: &libxml::tree::Node,
	type_code: Option<&str>,
) -> Result<(Option<String>, Option<String>)> {
	let value = first_attr(xpath, node, "hl7:id", "extension");
	let null_flavor = first_attr(xpath, node, "hl7:id", "nullFlavor");
	match type_code {
		Some("1") => import_constraint::string(
			"gpMedicalRecordNumber",
			value.as_deref(),
			null_flavor.as_deref(),
			input_contracts::generated::d::d_1_1_1,
		)?,
		Some("2") => import_constraint::string(
			"specialistRecordNumber",
			value.as_deref(),
			null_flavor.as_deref(),
			input_contracts::generated::d::d_1_1_2,
		)?,
		Some("3") => import_constraint::string(
			"hospitalRecordNumber",
			value.as_deref(),
			null_flavor.as_deref(),
			input_contracts::generated::d::d_1_1_3,
		)?,
		Some("4") => import_constraint::string(
			"investigationNumber",
			value.as_deref(),
			null_flavor.as_deref(),
			input_contracts::generated::d::d_1_1_4,
		)?,
		_ => {}
	}
	Ok((value, null_flavor))
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
		let (meddra_version, meddra_code) = read_d_7_1_r_1(&mut xpath, &node)?;
		let (start_date, start_date_null_flavor) =
			read_d_7_1_r_2(&mut xpath, &node)?;
		let (continuing, continuing_null_flavor) =
			read_d_7_1_r_3(&mut xpath, &node)?;
		let (end_date, end_date_null_flavor) = read_d_7_1_r_4(&mut xpath, &node)?;
		let comments = read_d_7_1_r_5(&mut xpath, &node)?;
		let family_history = read_d_7_1_r_6(&mut xpath, &node)?;
		items.push(MedicalHistoryImport {
			meddra_version,
			meddra_code,
			start_date,
			start_date_null_flavor,
			continuing,
			continuing_null_flavor,
			end_date,
			end_date_null_flavor,
			comments,
			family_history,
		});
	}
	Ok(items)
}

/// e2b:D.7.1.r.1a
/// e2b:D.7.1.r.1b
fn read_d_7_1_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	Ok((
		input_string(
			first_attr(xpath, node, "hl7:code", "codeSystemVersion"),
			"medicalHistoryEpisodes[].meddraVersion",
			input_contracts::generated::d::d_7_1_r_1a,
		)?,
		input_string(
			first_attr(xpath, node, "hl7:code", "code"),
			"medicalHistoryEpisodes[].meddraCode",
			input_contracts::generated::d::d_7_1_r_1b,
		)?,
	))
}

/// e2b:D.7.1.r.2
fn read_d_7_1_r_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	input_ts(
		first_attr(xpath, node, "hl7:effectiveTime/hl7:low", "value"),
		first_attr(xpath, node, "hl7:effectiveTime/hl7:low", "nullFlavor"),
		"medicalHistoryEpisodes[].startDate",
		input_contracts::generated::d::d_7_1_r_2,
	)
}

/// e2b:D.7.1.r.3
fn read_d_7_1_r_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<bool>, Option<String>)> {
	let path =
		"hl7:inboundRelationship/hl7:observation[hl7:code[@code='13']]/hl7:value";
	let value = parse_bool_attr(xpath, node, path, "value");
	let null_flavor = first_attr(xpath, node, path, "nullFlavor");
	import_constraint::boolean(
		"medicalHistoryEpisodes[].continuing",
		value,
		null_flavor.as_deref(),
		input_contracts::generated::d::d_7_1_r_3,
	)?;
	import_constraint::string(
		"medicalHistoryEpisodes[].continuingNullFlavor",
		None,
		None,
		input_contracts::generated::d::d_7_1_r_3,
	)?;
	Ok((value, null_flavor))
}

/// e2b:D.7.1.r.4
fn read_d_7_1_r_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	input_ts(
		first_attr(xpath, node, "hl7:effectiveTime/hl7:high", "value"),
		first_attr(xpath, node, "hl7:effectiveTime/hl7:high", "nullFlavor"),
		"medicalHistoryEpisodes[].endDate",
		input_contracts::generated::d::d_7_1_r_4,
	)
}

/// e2b:D.7.1.r.5
fn read_d_7_1_r_5(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	input_string(first_text(
		xpath,
		node,
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='10']]/hl7:value",
	), "medicalHistoryEpisodes[].comments", input_contracts::generated::d::d_7_1_r_5)
}

/// e2b:D.7.1.r.6
fn read_d_7_1_r_6(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<bool>> {
	let value = parse_bool_attr(
		xpath,
		node,
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='38']]/hl7:value",
		"value",
	);
	import_constraint::boolean(
		"medicalHistoryEpisodes[].familyHistory",
		value,
		None,
		input_contracts::generated::d::d_7_1_r_6,
	)?;
	Ok(value)
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
		let (drug_name, drug_name_null_flavor) = read_d_8_r_1(&mut xpath, &node)?;
		let (mfds_medicinal_product_version, mfds_medicinal_product_id) =
			read_d_8_r_1_kr(&mut xpath, &node)?;
		let (
			mpid_version,
			mpid,
			mpid_source_code_system,
			mpid_source_code_system_version,
		) = read_d_8_r_2(&mut xpath, &node)?;
		let (phpid_version, phpid) = read_d_8_r_3(&mut xpath, &node)?;
		let (start_date, start_date_null_flavor) = read_d_8_r_4(&mut xpath, &node)?;
		let (end_date, end_date_null_flavor) = read_d_8_r_5(&mut xpath, &node)?;
		let (indication_meddra_version, indication_meddra_code) =
			read_d_8_r_6(&mut xpath, &node)?;
		let (reaction_meddra_version, reaction_meddra_code) =
			read_d_8_r_7(&mut xpath, &node)?;
		items.push(PastDrugHistoryImport {
			drug_name,
			drug_name_null_flavor,
			mpid,
			mpid_version,
			mpid_source_code_system,
			mpid_source_code_system_version,
			mfds_medicinal_product_version,
			mfds_medicinal_product_id,
			phpid,
			phpid_version,
			start_date,
			start_date_null_flavor,
			end_date,
			end_date_null_flavor,
			indication_meddra_version,
			indication_meddra_code,
			reaction_meddra_version,
			reaction_meddra_code,
		});
	}
	Ok(items)
}

const PRODUCT: &str = "hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct";

/// e2b:D.8.r.1
fn read_d_8_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let path = format!("{PRODUCT}/hl7:name");
	let value = first_text(xpath, node, &path);
	let null_flavor = first_attr(xpath, node, &path, "nullFlavor");
	import_constraint::string(
		"drugName",
		value.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::d::d_8_r_1,
	)?;
	Ok((value, null_flavor))
}

/// e2b:D.8.r.1.KR.1a
/// e2b:D.8.r.1.KR.1b
fn read_d_8_r_1_kr(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let path = format!(
		"{PRODUCT}/hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.10.2.1']"
	);
	Ok((
		input_string(
			first_attr(xpath, node, &path, "codeSystemVersion"),
			"mfdsMedicinalProductVersion",
			input_contracts::generated::d::mfds_d_8_r_1_kr_1a,
		)?,
		input_string(
			first_attr(xpath, node, &path, "code"),
			"mfdsMedicinalProductId",
			input_contracts::generated::d::mfds_d_8_r_1_kr_1b,
		)?,
	))
}

/// e2b:D.8.r.2a
/// e2b:D.8.r.2b
fn read_d_8_r_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
)> {
	let base = format!("{PRODUCT}/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]");
	let fda_code =
		format!("{PRODUCT}/hl7:code[@codeSystem='2.16.840.1.113883.6.69']");
	if let Some(mpid) = first_attr(xpath, node, &fda_code, "code") {
		input_string(
			Some(mpid.clone()),
			"mpid",
			input_contracts::generated::d::d_8_r_2b,
		)?;
		let source_version = input_string(
			first_attr(xpath, node, &fda_code, "codeSystemVersion"),
			"mpidSourceCodeSystemVersion",
			input_contracts::generated::d::mfds_d_8_r_1_kr_1a,
		)?;
		return Ok((
			None,
			Some(mpid),
			Some("2.16.840.1.113883.6.69".to_string()),
			source_version,
		));
	}
	Ok((
		input_string(
			first_value(xpath, node, &format!("{base}/hl7:code/@codeSystemVersion"))
				.or_else(|| first_attr(xpath, node, &fda_code, "codeSystemVersion")),
			"mpidVersion",
			input_contracts::generated::d::d_8_r_2a,
		)?,
		input_string(
			first_value(xpath, node, &format!("{base}/hl7:id/@extension"))
				.or_else(|| first_attr(xpath, node, &fda_code, "code")),
			"mpid",
			input_contracts::generated::d::d_8_r_2b,
		)?,
		None,
		None,
	))
}

/// e2b:D.8.r.3a
/// e2b:D.8.r.3b
fn read_d_8_r_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let base = format!("({PRODUCT}/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]");
	Ok((
		input_string(
			first_value(
				xpath,
				node,
				&format!("{base}/hl7:code/@codeSystemVersion)[1]"),
			),
			"phpidVersion",
			input_contracts::generated::d::d_8_r_3a,
		)?,
		input_string(
			first_value(xpath, node, &format!("{base}/hl7:id/@extension)[1]")),
			"phpid",
			input_contracts::generated::d::d_8_r_3b,
		)?,
	))
}

/// e2b:D.8.r.4
fn read_d_8_r_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<Date>, Option<String>)> {
	input_date(
		first_attr(xpath, node, "hl7:effectiveTime/hl7:low", "value"),
		first_attr(xpath, node, "hl7:effectiveTime/hl7:low", "nullFlavor"),
		"startDate",
		input_contracts::generated::d::d_8_r_4,
	)
}

/// e2b:D.8.r.5
fn read_d_8_r_5(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<Date>, Option<String>)> {
	input_date(
		first_attr(xpath, node, "hl7:effectiveTime/hl7:high", "value"),
		first_attr(xpath, node, "hl7:effectiveTime/hl7:high", "nullFlavor"),
		"endDate",
		input_contracts::generated::d::d_8_r_5,
	)
}

fn read_meddra_pair(
	xpath: &mut Context,
	node: &libxml::tree::Node,
	relationship: &str,
	version_field: &str,
	code_field: &str,
	version_check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
	code_check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<(Option<String>, Option<String>)> {
	let path = format!("hl7:outboundRelationship2[@typeCode='{relationship}']/hl7:observation/hl7:value");
	Ok((
		input_string(
			first_attr(xpath, node, &path, "codeSystemVersion"),
			version_field,
			version_check,
		)?,
		input_string(
			first_attr(xpath, node, &path, "code"),
			code_field,
			code_check,
		)?,
	))
}

/// e2b:D.8.r.6a
/// e2b:D.8.r.6b
fn read_d_8_r_6(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_meddra_pair(
		xpath,
		node,
		"RSON",
		"indicationMeddraVersion",
		"indicationMeddraCode",
		input_contracts::generated::d::d_8_r_6a,
		input_contracts::generated::d::d_8_r_6b,
	)
}

/// e2b:D.8.r.7a
/// e2b:D.8.r.7b
fn read_d_8_r_7(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	read_meddra_pair(
		xpath,
		node,
		"CAUS",
		"reactionMeddraVersion",
		"reactionMeddraCode",
		input_contracts::generated::d::d_8_r_7a,
		input_contracts::generated::d::d_8_r_7b,
	)
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

	let (date_of_death, date_of_death_null_flavor) = read_d_9_1(&mut xpath)?;
	let (autopsy_performed, autopsy_performed_null_flavor) = read_d_9_3(&mut xpath)?;

	let mut reported_causes = Vec::new();
	let reported_nodes = xpath
		.findnodes("//hl7:observation[hl7:code[@code='32']]/hl7:value", None)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query reported causes of death".to_string(),
			line: None,
			column: None,
		})?;
	for node in reported_nodes {
		let (meddra_version, meddra_code, comments) =
			read_d_9_2_r(&mut xpath, &node)?;
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
		let (meddra_version, meddra_code, comments) =
			read_d_9_4_r(&mut xpath, &node)?;
		autopsy_causes.push(DeathCauseImport {
			meddra_version,
			meddra_code,
			comments,
		});
	}

	if date_of_death.is_none()
		&& date_of_death_null_flavor.is_none()
		&& autopsy_performed.is_none()
		&& autopsy_performed_null_flavor.is_none()
		&& reported_causes.is_empty()
		&& autopsy_causes.is_empty()
	{
		return Ok(None);
	}

	Ok(Some(DeathImport {
		date_of_death,
		date_of_death_null_flavor,
		autopsy_performed,
		autopsy_performed_null_flavor,
		reported_causes,
		autopsy_causes,
	}))
}

/// e2b:D.9.1
fn read_d_9_1(xpath: &mut Context) -> Result<(Option<Date>, Option<String>)> {
	input_date(
		first_value_root(xpath, "//hl7:deceasedTime/@value"),
		first_value_root(xpath, "//hl7:deceasedTime/@nullFlavor"),
		"patientDeath.dateOfDeath",
		input_contracts::generated::d::d_9_1,
	)
}

/// e2b:D.9.3
fn read_d_9_3(xpath: &mut Context) -> Result<(Option<bool>, Option<String>)> {
	let value = parse_bool_with_yes_no(
		first_value_root(
			xpath,
			"//hl7:observation[hl7:code[@code='5']]/hl7:value/@value",
		)
		.as_deref(),
	);
	let null_flavor = first_value_root(
		xpath,
		"//hl7:observation[hl7:code[@code='5']]/hl7:value/@nullFlavor",
	);
	import_constraint::boolean(
		"patientDeath.autopsyPerformed",
		value,
		null_flavor.as_deref(),
		input_contracts::generated::d::d_9_3,
	)?;
	import_constraint::string(
		"patientDeath.autopsyPerformedNullFlavor",
		None,
		None,
		input_contracts::generated::d::d_9_3,
	)?;
	Ok((value, null_flavor))
}

/// e2b:D.9.2.r.1a
/// e2b:D.9.2.r.1b
/// e2b:D.9.2.r.2
fn read_d_9_2_r(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
	Ok((
		input_string(
			node.get_attribute("codeSystemVersion"),
			"patientDeath.reportedCausesOfDeath[].meddraVersion",
			input_contracts::generated::d::d_9_2_r_1a,
		)?,
		input_string(
			node.get_attribute("code"),
			"patientDeath.reportedCausesOfDeath[].meddraCode",
			input_contracts::generated::d::d_9_2_r_1b,
		)?,
		input_string(
			first_text(xpath, node, "hl7:originalText"),
			"patientDeath.reportedCausesOfDeath[].causeText",
			input_contracts::generated::d::d_9_2_r_2,
		)?,
	))
}

/// e2b:D.9.4.r.1a
/// e2b:D.9.4.r.1b
/// e2b:D.9.4.r.2
fn read_d_9_4_r(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
	Ok((
		input_string(
			node.get_attribute("codeSystemVersion"),
			"patientDeath.autopsyCausesOfDeath[].meddraVersion",
			input_contracts::generated::d::d_9_4_r_1a,
		)?,
		input_string(
			node.get_attribute("code"),
			"patientDeath.autopsyCausesOfDeath[].meddraCode",
			input_contracts::generated::d::d_9_4_r_1b,
		)?,
		input_string(
			first_text(xpath, node, "hl7:originalText"),
			"patientDeath.autopsyCausesOfDeath[].causeText",
			input_contracts::generated::d::d_9_4_r_2,
		)?,
	))
}

/// e2b:D.10.1
fn read_d_10_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let value = first_text(xpath, node, "hl7:associatedPerson/hl7:name");
	let null_flavor =
		first_attr(xpath, node, "hl7:associatedPerson/hl7:name", "nullFlavor");
	import_constraint::string(
		"parentInformation.parentIdentification",
		value.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::d::d_10_1,
	)?;
	import_constraint::string(
		"parentInformation.parentIdentificationNullFlavor",
		None,
		None,
		input_contracts::generated::d::d_10_1,
	)?;
	Ok((value, null_flavor))
}

/// e2b:D.10.2.1
fn read_d_10_2_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<Date>, Option<String>)> {
	let path = "hl7:associatedPerson/hl7:birthTime";
	input_date(
		first_attr(xpath, node, path, "value"),
		first_attr(xpath, node, path, "nullFlavor"),
		"parentInformation.parentBirthDate",
		input_contracts::generated::d::d_10_2_1,
	)
}

/// e2b:D.10.2.2a
fn read_d_10_2_2a(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<Decimal>> {
	let path = "hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value";
	input_number(
		first_attr(xpath, node, path, "value"),
		"parentInformation.parentAge.value",
		input_contracts::generated::d::d_10_2_2a,
	)
}

/// e2b:D.10.2.2b
fn read_d_10_2_2b(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	input_string(
		first_attr(
			xpath,
			node,
			"hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
			"unit",
		),
		"parentInformation.parentAge.unit",
		input_contracts::generated::d::d_10_2_2b,
	)
}

/// e2b:D.10.3
fn read_d_10_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<Date>, Option<String>)> {
	let path = "hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value";
	input_date(
		first_attr(xpath, node, path, "value"),
		first_attr(xpath, node, path, "nullFlavor"),
		"parentInformation.parentLastMenstrualPeriodDate",
		input_contracts::generated::d::d_10_3,
	)
}

/// e2b:D.10.4
fn read_d_10_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<Decimal>> {
	input_number(
		first_attr(
			xpath,
			node,
			"hl7:subjectOf2/hl7:observation[hl7:code[@code='7']]/hl7:value",
			"value",
		),
		"parentInformation.parentWeight.value",
		input_contracts::generated::d::d_10_4,
	)
}

/// e2b:D.10.5
fn read_d_10_5(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<Decimal>> {
	input_number(
		first_attr(
			xpath,
			node,
			"hl7:subjectOf2/hl7:observation[hl7:code[@code='17']]/hl7:value",
			"value",
		),
		"parentInformation.parentHeight.value",
		input_contracts::generated::d::d_10_5,
	)
}

/// e2b:D.10.6
fn read_d_10_6(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let path = "hl7:associatedPerson/hl7:administrativeGenderCode";
	let value = first_attr(xpath, node, path, "code");
	let null_flavor = first_attr(xpath, node, path, "nullFlavor");
	import_constraint::string(
		"parentInformation.parentSex",
		value.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::d::d_10_6,
	)?;
	import_constraint::string(
		"parentInformation.parentSexNullFlavor",
		None,
		None,
		input_contracts::generated::d::d_10_6,
	)?;
	Ok((value, null_flavor))
}

/// e2b:D.10.7.2
fn read_d_10_7_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	input_string(first_text(xpath, node, "hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value"), "parentInformation.medicalHistoryText", input_contracts::generated::d::d_10_7_2)
}

/// e2b:D.10.7.1.r.1a
/// e2b:D.10.7.1.r.1b
fn read_d_10_7_1_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	Ok((
		input_string(
			first_attr(xpath, node, "hl7:code", "codeSystemVersion"),
			"parentInformation.medicalHistoryEpisodes[].meddraVersion",
			input_contracts::generated::d::d_10_7_1_r_1a,
		)?,
		input_string(
			first_attr(xpath, node, "hl7:code", "code"),
			"parentInformation.medicalHistoryEpisodes[].meddraCode",
			input_contracts::generated::d::d_10_7_1_r_1b,
		)?,
	))
}

/// e2b:D.10.7.1.r.2
fn read_d_10_7_1_r_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<Date>, Option<String>)> {
	input_date(
		first_attr(xpath, node, "hl7:effectiveTime/hl7:low", "value"),
		first_attr(xpath, node, "hl7:effectiveTime/hl7:low", "nullFlavor"),
		"parentInformation.medicalHistoryEpisodes[].startDate",
		input_contracts::generated::d::d_10_7_1_r_2,
	)
}

/// e2b:D.10.7.1.r.3
fn read_d_10_7_1_r_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<bool>, Option<String>)> {
	let path =
		"hl7:inboundRelationship/hl7:observation[hl7:code[@code='13']]/hl7:value";
	let value = parse_bool_attr(xpath, node, path, "value");
	let null_flavor = first_attr(xpath, node, path, "nullFlavor");
	import_constraint::boolean(
		"parentInformation.medicalHistoryEpisodes[].continuing",
		value,
		null_flavor.as_deref(),
		input_contracts::generated::d::d_10_7_1_r_3,
	)?;
	import_constraint::string(
		"parentInformation.medicalHistoryEpisodes[].continuingNullFlavor",
		None,
		None,
		input_contracts::generated::d::d_10_7_1_r_3,
	)?;
	Ok((value, null_flavor))
}

/// e2b:D.10.7.1.r.4
fn read_d_10_7_1_r_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<Date>, Option<String>)> {
	input_date(
		first_attr(xpath, node, "hl7:effectiveTime/hl7:high", "value"),
		first_attr(xpath, node, "hl7:effectiveTime/hl7:high", "nullFlavor"),
		"parentInformation.medicalHistoryEpisodes[].endDate",
		input_contracts::generated::d::d_10_7_1_r_4,
	)
}

/// e2b:D.10.7.1.r.5
fn read_d_10_7_1_r_5(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	input_string(first_text(
		xpath,
		node,
		"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='10']]/hl7:value",
	), "parentInformation.medicalHistoryEpisodes[].comments", input_contracts::generated::d::d_10_7_1_r_5)
}

/// e2b:D.10.8.r.1
fn read_d_10_8_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<Option<String>> {
	let path = format!("{PRODUCT}/hl7:name");
	input_string(
		first_text(xpath, node, &path),
		"parentInformation.pastDrugHistory[].drugName",
		input_contracts::generated::d::d_10_8_r_1,
	)
}

/// e2b:D.10.8.r.1.KR.1a
/// e2b:D.10.8.r.1.KR.1b
fn read_d_10_8_r_1_kr(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let path = format!("{PRODUCT}/hl7:code");
	Ok((
		input_string(
			first_attr(xpath, node, &path, "codeSystemVersion"),
			"parentInformation.pastDrugHistory[].mfdsMedicinalProductVersion",
			input_contracts::generated::d::mfds_d_10_8_r_1_kr_1a,
		)?,
		input_string(
			first_attr(xpath, node, &path, "code"),
			"parentInformation.pastDrugHistory[].mfdsMedicinalProductId",
			input_contracts::generated::d::mfds_d_10_8_r_1_kr_1b,
		)?,
	))
}

/// e2b:D.10.8.r.2a
/// e2b:D.10.8.r.2b
fn read_d_10_8_r_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let base = format!("{PRODUCT}/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]");
	Ok((
		input_string(
			first_value(xpath, node, &format!("{base}/hl7:code/@codeSystemVersion")),
			"parentInformation.pastDrugHistory[].mpidVersion",
			input_contracts::generated::d::d_10_8_r_2a,
		)?,
		input_string(
			first_value(xpath, node, &format!("{base}/hl7:id/@extension")),
			"parentInformation.pastDrugHistory[].mpid",
			input_contracts::generated::d::d_10_8_r_2b,
		)?,
	))
}

/// e2b:D.10.8.r.3a
/// e2b:D.10.8.r.3b
fn read_d_10_8_r_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let base = format!("({PRODUCT}/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]");
	Ok((
		input_string(
			first_value(
				xpath,
				node,
				&format!("{base}/hl7:code/@codeSystemVersion)[1]"),
			),
			"parentInformation.pastDrugHistory[].phpidVersion",
			input_contracts::generated::d::d_10_8_r_3a,
		)?,
		input_string(
			first_value(xpath, node, &format!("{base}/hl7:id/@extension)[1]")),
			"parentInformation.pastDrugHistory[].phpid",
			input_contracts::generated::d::d_10_8_r_3b,
		)?,
	))
}

/// e2b:D.10.8.r.4
fn read_d_10_8_r_4(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<Date>, Option<String>)> {
	input_date(
		first_attr(xpath, node, "hl7:effectiveTime/hl7:low", "value"),
		first_attr(xpath, node, "hl7:effectiveTime/hl7:low", "nullFlavor"),
		"parentInformation.pastDrugHistory[].startDate",
		input_contracts::generated::d::d_10_8_r_4,
	)
}

/// e2b:D.10.8.r.5
fn read_d_10_8_r_5(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<Date>, Option<String>)> {
	input_date(
		first_attr(xpath, node, "hl7:effectiveTime/hl7:high", "value"),
		first_attr(xpath, node, "hl7:effectiveTime/hl7:high", "nullFlavor"),
		"parentInformation.pastDrugHistory[].endDate",
		input_contracts::generated::d::d_10_8_r_5,
	)
}

/// e2b:D.10.8.r.6a
/// e2b:D.10.8.r.6b
fn read_d_10_8_r_6(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let path =
		"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value";
	Ok((
		input_string(
			first_attr(xpath, node, path, "codeSystemVersion"),
			"parentInformation.pastDrugHistory[].indicationMeddraVersion",
			input_contracts::generated::d::d_10_8_r_6a,
		)?,
		input_string(
			first_attr(xpath, node, path, "code"),
			"parentInformation.pastDrugHistory[].indicationMeddraCode",
			input_contracts::generated::d::d_10_8_r_6b,
		)?,
	))
}

/// e2b:D.10.8.r.7a
/// e2b:D.10.8.r.7b
fn read_d_10_8_r_7(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let path =
		"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value";
	Ok((
		input_string(
			first_attr(xpath, node, path, "codeSystemVersion"),
			"parentInformation.pastDrugHistory[].reactionMeddraVersion",
			input_contracts::generated::d::d_10_8_r_7a,
		)?,
		input_string(
			first_attr(xpath, node, path, "code"),
			"parentInformation.pastDrugHistory[].reactionMeddraCode",
			input_contracts::generated::d::d_10_8_r_7b,
		)?,
	))
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
	let Some(node) = nodes.first() else {
		return Ok(None);
	};

	let (parent_identification, parent_identification_null_flavor) =
		read_d_10_1(&mut xpath, node)?;
	let (parent_birth_date, parent_birth_date_null_flavor) =
		read_d_10_2_1(&mut xpath, node)?;
	let parent_age = read_d_10_2_2a(&mut xpath, node)?;
	let parent_age_unit = read_d_10_2_2b(&mut xpath, node)?;
	let (last_menstrual_period_date, last_menstrual_period_date_null_flavor) =
		read_d_10_3(&mut xpath, node)?;
	let weight_kg = read_d_10_4(&mut xpath, node)?;
	let height_cm = read_d_10_5(&mut xpath, node)?;
	let (sex, sex_null_flavor) = read_d_10_6(&mut xpath, node)?;
	let medical_history_text = read_d_10_7_2(&mut xpath, node)?;

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
		let (meddra_version, meddra_code) = read_d_10_7_1_r_1(&mut xpath, &obs)?;
		let (start_date, start_date_null_flavor) =
			read_d_10_7_1_r_2(&mut xpath, &obs)?;
		let (continuing, continuing_null_flavor) =
			read_d_10_7_1_r_3(&mut xpath, &obs)?;
		let (end_date, end_date_null_flavor) = read_d_10_7_1_r_4(&mut xpath, &obs)?;
		let comments = read_d_10_7_1_r_5(&mut xpath, &obs)?;
		medical_history.push(ParentMedicalHistoryImport {
			meddra_version,
			meddra_code,
			start_date,
			start_date_null_flavor,
			continuing,
			continuing_null_flavor,
			end_date,
			end_date_null_flavor,
			comments,
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
		let drug_name = read_d_10_8_r_1(&mut xpath, &obs)?;
		let (mpid_version, mpid) = read_d_10_8_r_2(&mut xpath, &obs)?;
		let (mfds_medicinal_product_version, mfds_medicinal_product_id) =
			read_d_10_8_r_1_kr(&mut xpath, &obs)?;
		let (start_date, start_date_null_flavor) =
			read_d_10_8_r_4(&mut xpath, &obs)?;
		let (end_date, end_date_null_flavor) = read_d_10_8_r_5(&mut xpath, &obs)?;
		let (indication_meddra_version, indication_meddra_code) =
			read_d_10_8_r_6(&mut xpath, &obs)?;
		let (reaction_meddra_version, reaction_meddra_code) =
			read_d_10_8_r_7(&mut xpath, &obs)?;
		let (phpid_version, phpid) = read_d_10_8_r_3(&mut xpath, &obs)?;
		past_drugs.push(PastDrugHistoryImport {
			drug_name,
			drug_name_null_flavor: None,
			mpid,
			mpid_version,
			mpid_source_code_system: None,
			mpid_source_code_system_version: None,
			mfds_medicinal_product_version,
			mfds_medicinal_product_id,
			phpid,
			phpid_version,
			start_date,
			start_date_null_flavor,
			end_date,
			end_date_null_flavor,
			indication_meddra_version,
			indication_meddra_code,
			reaction_meddra_version,
			reaction_meddra_code,
		});
	}

	Ok(Some(ParentImport {
		parent_identification,
		parent_identification_null_flavor,
		parent_birth_date,
		parent_birth_date_null_flavor,
		parent_age,
		parent_age_unit,
		last_menstrual_period_date,
		last_menstrual_period_date_null_flavor,
		weight_kg,
		height_cm,
		sex,
		sex_null_flavor,
		medical_history_text,
		medical_history,
		past_drugs,
	}))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_parent_leaves_age_null_flavor_to_business_validation() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <primaryRole><player1><role><code code="PRN"/>
    <subjectOf2><observation><code code="3"/><value xsi:type="PQ" nullFlavor="UNK"/></observation></subjectOf2>
  </role></player1></primaryRole>
</MCCI_IN200100UV01>"#;

		assert!(parse_parent_information(xml).is_ok());
	}

	#[test]
	fn preserves_partial_medical_history_ts() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><primaryRole><subjectOf2><organizer><code code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><observation><code codeSystem="2.16.840.1.113883.6.163" code="100"/><effectiveTime xsi:type="IVL_TS"><low value="2018"/><high value="201904"/></effectiveTime></observation></component></organizer></subjectOf2></primaryRole></MCCI_IN200100UV01>"#;
		let history = parse_medical_history(xml).expect("parse");
		assert_eq!(history[0].start_date.as_deref(), Some("2018"));
		assert_eq!(history[0].end_date.as_deref(), Some("201904"));
	}

	#[test]
	fn parse_parent_leaves_drug_name_null_flavor_to_business_validation() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <primaryRole><player1><role><code code="PRN"/>
    <subjectOf2><organizer><code code="2"/><component><substanceAdministration>
      <consumable><instanceOfKind><kindOfProduct><name nullFlavor="UNK"/></kindOfProduct></instanceOfKind></consumable>
    </substanceAdministration></component></organizer></subjectOf2>
  </role></player1></primaryRole>
</MCCI_IN200100UV01>"#;

		assert!(parse_parent_information(xml).is_ok());
	}

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
							<code code="KR-DH-ID" codeSystem="2.16.840.1.113883.3.989.5.1.10.2.1" codeSystemVersion="KR-DH-V1"/>
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
	fn parse_d_8_r_2_accepts_fda_ndc_code() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><organizer><code code="2" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><code code="59762-2858" codeSystem="2.16.840.1.113883.6.69" codeSystemVersion="201411011202"/><name>CureAll</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></MCCI_IN200100UV01>"#;

		let past_drugs = parse_past_drug_history(xml).expect("parse FDA D.8");
		assert_eq!(past_drugs[0].mpid.as_deref(), Some("59762-2858"));
		assert_eq!(past_drugs[0].mpid_version, None);
		assert_eq!(
			past_drugs[0].mpid_source_code_system.as_deref(),
			Some("2.16.840.1.113883.6.69")
		);
		assert_eq!(
			past_drugs[0].mpid_source_code_system_version.as_deref(),
			Some("201411011202")
		);
		assert_eq!(past_drugs[0].mfds_medicinal_product_id, None);
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
