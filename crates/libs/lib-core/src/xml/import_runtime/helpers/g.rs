use crate::model::drug::{
	structured_fda_device_info_to_json, DrugAdditionalInfoCodeEntry,
	FdaDeviceCodeEntry, FdaDeviceInfoData,
};
use crate::xml::error::Error;
use crate::xml::import_runtime::shared::{
	clamp_str, first_attr, first_text, first_value, normalize_code, normalize_code3,
	normalize_iso2, parse_date, parse_time, parse_uuid_opt,
};
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Node;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use sqlx::types::time::{Date, Time};
use sqlx::types::Uuid;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct DrugSubstanceImport {
	pub(crate) substance_name: Option<String>,
	pub(crate) substance_termid: Option<String>,
	pub(crate) substance_termid_version: Option<String>,
	pub(crate) strength_value: Option<Decimal>,
	pub(crate) strength_unit: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DrugDosageImport {
	pub(crate) dosage_text: Option<String>,
	pub(crate) frequency_value: Option<Decimal>,
	pub(crate) frequency_unit: Option<String>,
	pub(crate) number_of_units: Option<i32>,
	pub(crate) start_date: Option<Date>,
	pub(crate) start_time: Option<Time>,
	pub(crate) start_date_null_flavor: Option<String>,
	pub(crate) end_date: Option<Date>,
	pub(crate) end_time: Option<Time>,
	pub(crate) end_date_null_flavor: Option<String>,
	pub(crate) duration_value: Option<Decimal>,
	pub(crate) duration_unit: Option<String>,
	pub(crate) dose_value: Option<Decimal>,
	pub(crate) dose_unit: Option<String>,
	pub(crate) route: Option<String>,
	pub(crate) route_termid_version: Option<String>,
	pub(crate) dose_form: Option<String>,
	pub(crate) dose_form_termid: Option<String>,
	pub(crate) dose_form_termid_version: Option<String>,
	pub(crate) batch_lot: Option<String>,
	pub(crate) parent_route_termid: Option<String>,
	pub(crate) parent_route_termid_version: Option<String>,
	pub(crate) parent_route: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DrugIndicationImport {
	pub(crate) text: Option<String>,
	pub(crate) version: Option<String>,
	pub(crate) code: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DrugDeviceCharacteristicImport {
	pub(crate) code: Option<String>,
	pub(crate) code_system: Option<String>,
	pub(crate) code_display_name: Option<String>,
	pub(crate) value_type: Option<String>,
	pub(crate) value_value: Option<String>,
	pub(crate) value_code: Option<String>,
	pub(crate) value_code_system: Option<String>,
	pub(crate) value_display_name: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DrugImport {
	pub(crate) xml_id: Option<Uuid>,
	pub(crate) sequence_number: i32,
	pub(crate) medicinal_product: String,
	pub(crate) brand_name: Option<String>,
	pub(crate) drug_characterization: String,
	pub(crate) mpid: Option<String>,
	pub(crate) mpid_version: Option<String>,
	pub(crate) phpid: Option<String>,
	pub(crate) phpid_version: Option<String>,
	pub(crate) investigational_product_blinded: Option<bool>,
	pub(crate) obtain_drug_country: Option<String>,
	pub(crate) drug_authorization_number: Option<String>,
	pub(crate) manufacturer_name: Option<String>,
	pub(crate) manufacturer_country: Option<String>,
	pub(crate) batch_lot_number: Option<String>,
	pub(crate) cumulative_dose_first_reaction_value: Option<Decimal>,
	pub(crate) cumulative_dose_first_reaction_unit: Option<String>,
	pub(crate) gestation_period_exposure_value: Option<Decimal>,
	pub(crate) gestation_period_exposure_unit: Option<String>,
	pub(crate) dosage_text: Option<String>,
	pub(crate) action_taken: Option<String>,
	pub(crate) rechallenge: Option<String>,
	pub(crate) parent_route: Option<String>,
	pub(crate) parent_route_termid: Option<String>,
	pub(crate) parent_route_termid_version: Option<String>,
	pub(crate) parent_dosage_text: Option<String>,
	pub(crate) fda_additional_info_coded: Option<String>,
	pub(crate) fda_specialized_product_category: Option<String>,
	pub(crate) fda_device_brand_name: Option<String>,
	pub(crate) fda_common_device_name: Option<String>,
	pub(crate) fda_device_product_code: Option<String>,
	pub(crate) fda_device_manufacturer_name: Option<String>,
	pub(crate) fda_device_manufacturer_address: Option<String>,
	pub(crate) fda_device_manufacturer_city: Option<String>,
	pub(crate) fda_device_manufacturer_state: Option<String>,
	pub(crate) fda_device_manufacturer_country: Option<String>,
	pub(crate) fda_device_lot_number: Option<String>,
	pub(crate) fda_operator_of_device: Option<String>,
	pub(crate) substances: Vec<DrugSubstanceImport>,
	pub(crate) dosages: Vec<DrugDosageImport>,
	pub(crate) indications: Vec<DrugIndicationImport>,
	pub(crate) characteristics: Vec<DrugDeviceCharacteristicImport>,
}

#[derive(Debug)]
pub(crate) struct DrugObservationImport {
	pub(crate) drug_xml_id: Option<Uuid>,
	pub(crate) drug_sequence: i32,
	pub(crate) sequence_number: i32,
	pub(crate) reaction_xml_id: Option<Uuid>,
	pub(crate) administration_start_interval_value: Option<Decimal>,
	pub(crate) administration_start_interval_unit: Option<String>,
	pub(crate) last_dose_interval_value: Option<Decimal>,
	pub(crate) last_dose_interval_unit: Option<String>,
	pub(crate) reaction_recurred: Option<String>,
	pub(crate) rechallenge_action: Option<String>,
	pub(crate) recurrence_meddra_version: Option<String>,
	pub(crate) recurrence_meddra_code: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RelatednessImport {
	pub(crate) drug_xml_id: Option<Uuid>,
	pub(crate) reaction_xml_id: Option<Uuid>,
	pub(crate) source_of_assessment: Option<String>,
	pub(crate) method_of_assessment: Option<String>,
	pub(crate) result_of_assessment: Option<String>,
}

fn build_xpath(xml: &[u8]) -> Result<(libxml::tree::Document, Context)> {
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
	let xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let _ =
		xpath.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
	Ok((doc, xpath))
}

fn normalize_characteristic_code(value: Option<&str>) -> String {
	value
		.unwrap_or("")
		.trim()
		.to_ascii_uppercase()
		.replace(['.', '_', '-'], "")
}

pub(crate) fn import_fda_device_info(
	drug: &DrugImport,
	characteristics: &[DrugDeviceCharacteristicImport],
) -> (Option<String>, Option<serde_json::Value>) {
	let mut info = FdaDeviceInfoData::default();
	let mut specialized_product_category =
		drug.fda_specialized_product_category.clone();
	info.device_brand_name = drug.fda_device_brand_name.clone();
	info.common_device_name = drug.fda_common_device_name.clone();
	info.device_product_code = drug.fda_device_product_code.clone();
	info.manufacturer_name = drug.fda_device_manufacturer_name.clone();
	info.manufacturer_address = drug.fda_device_manufacturer_address.clone();
	info.manufacturer_city = drug.fda_device_manufacturer_city.clone();
	info.manufacturer_state = drug.fda_device_manufacturer_state.clone();
	info.manufacturer_country = drug.fda_device_manufacturer_country.clone();
	info.device_lot_number = drug.fda_device_lot_number.clone();
	info.operator_of_device = drug.fda_operator_of_device.clone();

	for characteristic in characteristics {
		let normalized =
			normalize_characteristic_code(characteristic.code.as_deref());
		let display = characteristic
			.code_display_name
			.as_deref()
			.unwrap_or("")
			.trim()
			.to_ascii_lowercase();
		let code_value = characteristic
			.value_code
			.as_deref()
			.or(characteristic.value_value.as_deref())
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(str::to_string);
		let text_value = characteristic
			.value_value
			.as_deref()
			.or(characteristic.value_code.as_deref())
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(str::to_string);

		match normalized.as_str() {
			"FDAGK101" | "C94031"
				if display == "fda specialized product category" =>
			{
				specialized_product_category = code_value
			}
			"FDAGK12R1" | "C54026" => {
				info.malfunction = code_value
					.as_deref()
					.map(|value| matches!(value, "1" | "true" | "TRUE" | "True"))
			}
			"FDAGK12R2R" | "C54592" => {
				info.follow_up_types.push(FdaDeviceCodeEntry {
					value_code: code_value,
				})
			}
			"FDAGK12R3R" | "C54451" => {
				info.device_problem_codes.push(FdaDeviceCodeEntry {
					value_code: code_value,
				})
			}
			"FDAGK12R4" => info.device_brand_name = text_value,
			"FDAGK12R5" => info.common_device_name = text_value,
			"FDAGK12R6" => info.device_product_code = code_value,
			"FDAGK12R71A" => info.manufacturer_name = text_value,
			"FDAGK12R71B" => info.manufacturer_address = text_value,
			"FDAGK12R71C" => info.manufacturer_city = text_value,
			"FDAGK12R71D" => info.manufacturer_state = text_value,
			"FDAGK12R71E" => info.manufacturer_country = code_value,
			"FDAGK12R8" | "C54595" => info.device_usage = code_value,
			"FDAGK12R9" => info.device_lot_number = text_value,
			"FDAGK12R10" | "1" | "2" | "3" | "4" => {
				if characteristic.value_code.is_some()
					|| display == "health professional"
				{
					info.operator_of_device = Some(
						characteristic
							.value_code
							.clone()
							.or_else(|| characteristic.code.clone())
							.unwrap_or_default(),
					)
				}
			}
			"FDAGK12R11R" | "C54594" => {
				info.remedial_actions.push(FdaDeviceCodeEntry {
					value_code: code_value,
				})
			}
			_ => {}
		}
	}

	(
		specialized_product_category,
		structured_fda_device_info_to_json(Some(info)),
	)
}

pub(crate) fn build_drug_additional_info_codes_json(
	code: Option<&str>,
) -> Option<serde_json::Value> {
	let value_code = code?.trim();
	if value_code.is_empty() {
		return None;
	}
	serde_json::to_value(vec![DrugAdditionalInfoCodeEntry {
		value_code: Some(value_code.to_string()),
	}])
	.ok()
}

#[allow(dead_code)]
pub(crate) fn parse_drugs(xml: &[u8]) -> Result<Vec<DrugImport>> {
	let (_doc, mut xpath) = build_xpath(xml)?;
	let drug_nodes = xpath
		.findnodes(
			"//hl7:subjectOf2/hl7:organizer[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query drug information".to_string(),
			line: None,
			column: None,
		})?;

	let mut imports: Vec<DrugImport> = Vec::new();
	for (idx, node) in drug_nodes.into_iter().enumerate() {
		let xml_id = parse_uuid_opt(first_attr(&mut xpath, &node, "hl7:id", "root"));
		let name1 = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name[1]",
		)
		.ok_or_else(|| Error::InvalidXml {
			message: format!(
				"ICH.G.k.2.2.REQUIRED: medicinal product name missing for drug index {}",
				idx + 1
			),
			line: None,
			column: None,
		})?;
		let name2 = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name[2]",
		);

		let drug_characterization = "1".to_string();
		let mpid = first_value(
			&mut xpath,
			&node,
			"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@code)[1]",
		);
		let mpid_version = clamp_str(
			first_value(
				&mut xpath,
				&node,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@codeSystemVersion)[1]",
			),
			10,
			"drug_information.mpid_version",
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
			"drug_information.phpid_version",
		);
		let investigational_product_blinded = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:subjectOf/hl7:observation[hl7:code[@code='G.k.2.5']]/hl7:value",
			"value",
		)
		.and_then(|v| match v.to_ascii_lowercase().as_str() {
			"true" | "1" => Some(true),
			"false" | "0" => Some(false),
			_ => None,
		});
		let drug_authorization_number = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:id",
			"extension",
		);
		let manufacturer_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:holder/hl7:role/hl7:playingOrganization/hl7:name",
		);
		let manufacturer_country = normalize_iso2(
			first_attr(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:author/hl7:territorialAuthority/hl7:territory/hl7:code",
				"code",
			),
			"drug_information.manufacturer_country",
		);
		let obtain_drug_country = normalize_iso2(
			first_text(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:subjectOf/hl7:productEvent/hl7:performer/hl7:assignedEntity/hl7:representedOrganization/hl7:addr/hl7:country",
			),
			"drug_information.obtain_drug_country",
		);
		let action_taken = normalize_code(
			first_attr(
				&mut xpath,
				&node,
				"hl7:inboundRelationship[@typeCode='CAUS']/hl7:act/hl7:code",
				"code",
			),
			&["1", "2", "3", "4", "5", "6"],
			"drug_information.action_taken",
		);
		if let Some(val) = action_taken.as_deref() {
			eprintln!("[import_e2b_xml] action_taken={val}");
		}
		let rechallenge = normalize_code(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='31']]/hl7:value",
				"code",
			),
			&["1", "2", "3", "4"],
			"drug_information.rechallenge",
		);
		let dosage_text = first_text(&mut xpath, &node, "hl7:text");
		let batch_lot_number = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText",
		);
		let cumulative_dose_first_reaction_value = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='SUMM']/hl7:observation[hl7:code[@code='14']]/hl7:value",
			"value",
		)
		.and_then(|v| v.parse::<Decimal>().ok());
		let cumulative_dose_first_reaction_unit = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='SUMM']/hl7:observation[hl7:code[@code='14']]/hl7:value",
			"unit",
		);
		let gestation_period_exposure_value = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='16']]/hl7:value",
			"value",
		)
		.and_then(|v| v.parse::<Decimal>().ok());
		let gestation_period_exposure_unit = normalize_code3(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='16']]/hl7:value",
				"unit",
			),
			"drug_information.gestation_period_exposure_unit",
		);
		let fda_additional_info_coded = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2[@typeCode='REFR']/hl7:observation[hl7:code[@code='9']]/hl7:value",
				"code",
			),
			10,
			"drug_information.fda_additional_info_coded",
		);
		let fda_specialized_product_category = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic[hl7:code[@displayName='FDA Specialized Product Category']]/hl7:value",
			"code",
		);
		let fda_device_brand_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:name[1]",
		);
		let fda_common_device_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:name[2]",
		);
		let fda_device_product_code = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:code",
			"code",
		);
		let fda_device_manufacturer_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:name",
		);
		let fda_device_manufacturer_address = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:streetAddressLine",
		);
		let fda_device_manufacturer_city = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:city",
		);
		let fda_device_manufacturer_state = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:state",
		);
		let fda_device_manufacturer_country = normalize_iso2(
			first_text(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:country",
			),
			"drug_information.fda_device_manufacturer_country",
		);
		let fda_device_lot_number = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText",
		);
		let fda_operator_of_device = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic[hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.2.1.1.6']]/hl7:code",
			"code",
		);
		let parent_route_termid_version = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value",
				"codeSystemVersion",
			),
			10,
			"drug_information.parent_route_termid_version",
		);
		let parent_route_termid = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value",
			"code",
		);
		let parent_dosage_text = first_text(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='REFR']/hl7:observation[hl7:code[@code='2']]/hl7:value",
		);
		let parent_route = first_text(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/hl7:originalText",
		);

		let substances = parse_substances(&mut xpath, &node);
		let dosages = parse_dosages(&mut xpath, &node);
		let indications = parse_indications(&mut xpath, &node);
		let characteristics = parse_characteristics(&mut xpath, &node);

		imports.push(DrugImport {
			xml_id,
			sequence_number: (idx + 1) as i32,
			medicinal_product: name1,
			brand_name: name2,
			drug_characterization,
			mpid,
			mpid_version,
			phpid,
			phpid_version,
			investigational_product_blinded,
			obtain_drug_country,
			drug_authorization_number,
			manufacturer_name,
			manufacturer_country,
			batch_lot_number,
			cumulative_dose_first_reaction_value,
			cumulative_dose_first_reaction_unit,
			gestation_period_exposure_value,
			gestation_period_exposure_unit,
			dosage_text,
			action_taken,
			rechallenge,
			parent_route,
			parent_route_termid,
			parent_route_termid_version,
			parent_dosage_text,
			fda_additional_info_coded,
			fda_specialized_product_category,
			fda_device_brand_name,
			fda_common_device_name,
			fda_device_product_code,
			fda_device_manufacturer_name,
			fda_device_manufacturer_address,
			fda_device_manufacturer_city,
			fda_device_manufacturer_state,
			fda_device_manufacturer_country,
			fda_device_lot_number,
			fda_operator_of_device,
			substances,
			dosages,
			indications,
			characteristics,
		});
	}

	Ok(imports)
}

#[allow(dead_code)]
fn parse_substances(xpath: &mut Context, node: &Node) -> Vec<DrugSubstanceImport> {
	let subs = xpath
		.findnodes(
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:ingredient",
			Some(node),
		)
		.unwrap_or_default();
	let mut substances = Vec::new();
	for sub in subs {
		let sub_name = first_text(xpath, &sub, "hl7:ingredientSubstance/hl7:name");
		let termid =
			first_attr(xpath, &sub, "hl7:ingredientSubstance/hl7:code", "code");
		let termid_version = clamp_str(
			first_attr(
				xpath,
				&sub,
				"hl7:ingredientSubstance/hl7:code",
				"codeSystemVersion",
			),
			10,
			"drug_active_substances.substance_termid_version",
		);
		let strength_value =
			first_attr(xpath, &sub, "hl7:quantity/hl7:numerator", "value")
				.and_then(|v| v.parse::<Decimal>().ok());
		let strength_unit =
			first_attr(xpath, &sub, "hl7:quantity/hl7:numerator", "unit");
		substances.push(DrugSubstanceImport {
			substance_name: sub_name,
			substance_termid: termid,
			substance_termid_version: termid_version,
			strength_value,
			strength_unit,
		});
	}
	substances
}

#[allow(dead_code)]
fn parse_dosages(xpath: &mut Context, node: &Node) -> Vec<DrugDosageImport> {
	let dosages = xpath
		.findnodes(
			"hl7:outboundRelationship2[@typeCode='COMP']/hl7:substanceAdministration",
			Some(node),
		)
		.unwrap_or_default();
	let mut dosage_list = Vec::new();
	for dose in dosages {
		let dosage_text = first_text(xpath, &dose, "hl7:text");
		let frequency_value = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@xsi:type='PIVL_TS']/hl7:period",
			"value",
		)
		.and_then(|v| v.parse::<Decimal>().ok());
		let frequency_unit = normalize_code3(
			first_attr(
				xpath,
				&dose,
				"hl7:effectiveTime/hl7:comp[@xsi:type='PIVL_TS']/hl7:period",
				"unit",
			),
			"dosage_information.frequency_unit",
		);
		let effective_time_null_flavor =
			first_attr(xpath, &dose, "hl7:effectiveTime", "nullFlavor");
		let number_of_units = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@xsi:type='PIVL_TS']/hl7:period",
			"value",
		)
		.and_then(|v| v.parse::<i32>().ok());
		let start_date = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:low",
			"value",
		)
		.and_then(parse_date);
		let start_time = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:low",
			"value",
		)
		.and_then(parse_time);
		let start_date_null_flavor = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:low",
			"nullFlavor",
		)
		.or_else(|| effective_time_null_flavor.clone());
		let end_date = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:high",
			"value",
		)
		.and_then(parse_date);
		let end_time = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:high",
			"value",
		)
		.and_then(parse_time);
		let end_date_null_flavor = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:high",
			"nullFlavor",
		)
		.or_else(|| effective_time_null_flavor.clone());
		let duration_value = first_attr(
			xpath,
			&dose,
			"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:width",
			"value",
		)
		.and_then(|v| v.parse::<Decimal>().ok());
		let duration_unit = normalize_code3(
			first_attr(
				xpath,
				&dose,
				"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:width",
				"unit",
			),
			"dosage_information.duration_unit",
		);
		let dose_value = first_attr(xpath, &dose, "hl7:doseQuantity", "value")
			.and_then(|v| v.parse::<Decimal>().ok());
		let dose_unit = first_attr(xpath, &dose, "hl7:doseQuantity", "unit");
		let route = normalize_code3(
			first_attr(xpath, &dose, "hl7:routeCode", "code"),
			"dosage_information.route_of_administration",
		);
		let route_termid_version = clamp_str(
			first_attr(xpath, &dose, "hl7:routeCode", "codeSystemVersion"),
			10,
			"dosage_information.route_termid_version",
		);
		let dose_form = first_text(
			xpath,
			&dose,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode/hl7:originalText",
		);
		let dose_form_termid = first_attr(
			xpath,
			&dose,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode",
			"code",
		);
		let dose_form_termid_version = clamp_str(
			first_attr(
				xpath,
				&dose,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode",
				"codeSystemVersion",
			),
			10,
			"dosage_information.dose_form_termid_version",
		);
		let batch_lot = first_text(
			xpath,
			&dose,
			"hl7:consumable/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText",
		);
		let parent_route_termid = first_attr(
			xpath,
			&dose,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value",
			"code",
		);
		let parent_route_termid_version = clamp_str(
			first_attr(
				xpath,
				&dose,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value",
				"codeSystemVersion",
			),
			10,
			"dosage_information.parent_route_termid_version",
		);
		let parent_route = first_text(
			xpath,
			&dose,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/hl7:originalText",
		);

		dosage_list.push(DrugDosageImport {
			dosage_text,
			frequency_value,
			frequency_unit,
			number_of_units,
			start_date,
			start_time,
			start_date_null_flavor,
			end_date,
			end_time,
			end_date_null_flavor,
			duration_value,
			duration_unit,
			dose_value,
			dose_unit,
			route,
			route_termid_version,
			dose_form,
			dose_form_termid,
			dose_form_termid_version,
			batch_lot,
			parent_route_termid,
			parent_route_termid_version,
			parent_route,
		});
	}
	dosage_list
}

#[allow(dead_code)]
fn parse_indications(xpath: &mut Context, node: &Node) -> Vec<DrugIndicationImport> {
	let inds = xpath
		.findnodes(
			"hl7:inboundRelationship[@typeCode='RSON']/hl7:observation/hl7:value",
			Some(node),
		)
		.unwrap_or_default();
	let mut indications = Vec::new();
	for ind in inds {
		let text = first_text(xpath, &ind, "hl7:originalText");
		let code = first_attr(xpath, &ind, ".", "code");
		let version = clamp_str(
			first_attr(xpath, &ind, ".", "codeSystemVersion"),
			10,
			"drug_indications.indication_meddra_version",
		);
		indications.push(DrugIndicationImport {
			text,
			version,
			code,
		});
	}
	indications
}

#[allow(dead_code)]
fn parse_characteristics(
	xpath: &mut Context,
	node: &Node,
) -> Vec<DrugDeviceCharacteristicImport> {
	let chars = xpath
		.findnodes(
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic",
			Some(node),
		)
		.unwrap_or_default();
	let mut characteristics = Vec::new();
	for ch in chars {
		let code = first_attr(xpath, &ch, "hl7:code", "code");
		let code_system = first_attr(xpath, &ch, "hl7:code", "codeSystem");
		let code_display_name = first_attr(xpath, &ch, "hl7:code", "displayName");
		let value_type = first_attr(xpath, &ch, "hl7:value", "xsi:type")
			.or_else(|| first_attr(xpath, &ch, "hl7:value", "type"));
		let value_value = first_attr(xpath, &ch, "hl7:value", "value");
		let value_code = first_attr(xpath, &ch, "hl7:value", "code");
		let value_code_system = first_attr(xpath, &ch, "hl7:value", "codeSystem");
		let value_display_name = first_attr(xpath, &ch, "hl7:value", "displayName");
		characteristics.push(DrugDeviceCharacteristicImport {
			code,
			code_system,
			code_display_name,
			value_type,
			value_value,
			value_code,
			value_code_system,
			value_display_name,
		});
	}
	characteristics
}

pub(crate) fn parse_drug_observations(
	xml: &[u8],
) -> Result<Vec<DrugObservationImport>> {
	let (_doc, mut xpath) = build_xpath(xml)?;
	let drug_nodes = xpath
		.findnodes(
			"//hl7:subjectOf2/hl7:organizer[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query drug information".to_string(),
			line: None,
			column: None,
		})?;

	let mut observations: Vec<DrugObservationImport> = Vec::new();
	for (didx, drug_node) in drug_nodes.into_iter().enumerate() {
		let drug_sequence = (didx + 1) as i32;
		let drug_xml_id =
			parse_uuid_opt(first_attr(&mut xpath, &drug_node, "hl7:id", "root"));
		let obs_nodes = xpath
			.findnodes(
				"hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='31']]",
				Some(&drug_node),
			)
			.map_err(|_| Error::InvalidXml {
				message: "Failed to query drug recurrence observations".to_string(),
				line: None,
				column: None,
			})?;
		let time_rels = xpath
			.findnodes(
				"hl7:outboundRelationship1[@typeCode='SAS' or @typeCode='SAE']",
				Some(&drug_node),
			)
			.map_err(|_| Error::InvalidXml {
				message: "Failed to query drug time intervals".to_string(),
				line: None,
				column: None,
			})?;
		let mut administration_start_map: HashMap<
			Uuid,
			(Option<Decimal>, Option<String>),
		> = HashMap::new();
		let mut last_dose_map: HashMap<Uuid, (Option<Decimal>, Option<String>)> =
			HashMap::new();
		for rel in time_rels {
			let rel_type = rel.get_attribute("typeCode");
			let reaction_id = parse_uuid_opt(first_attr(
				&mut xpath,
				&rel,
				"hl7:actReference/hl7:id",
				"root",
			));
			let value = first_attr(&mut xpath, &rel, "hl7:pauseQuantity", "value")
				.and_then(|v| v.parse::<Decimal>().ok());
			let unit = first_attr(&mut xpath, &rel, "hl7:pauseQuantity", "unit");
			if let Some(reaction_id) = reaction_id {
				if matches!(rel_type.as_deref(), Some("SAS")) {
					administration_start_map.insert(reaction_id, (value, unit));
				} else if matches!(rel_type.as_deref(), Some("SAE")) {
					last_dose_map.insert(reaction_id, (value, unit));
				}
			}
		}

		for (oidx, obs) in obs_nodes.into_iter().enumerate() {
			let sequence_number = (oidx + 1) as i32;
			let reaction_recurred = normalize_code(
				first_attr(&mut xpath, &obs, "hl7:value", "code"),
				&["1", "2", "3"],
				"drug_recurrence_information.reaction_recurred",
			);
			let reaction_xml_id = parse_uuid_opt(first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship1[@typeCode='REFR']/hl7:actReference/hl7:id",
				"root",
			));
			let (
				administration_start_interval_value,
				administration_start_interval_unit,
			) = if let Some(id) = reaction_xml_id {
				administration_start_map
					.get(&id)
					.cloned()
					.unwrap_or((None, None))
			} else if administration_start_map.len() == 1 {
				administration_start_map
					.values()
					.next()
					.cloned()
					.unwrap_or((None, None))
			} else {
				(None, None)
			};
			let (last_dose_interval_value, last_dose_interval_unit) =
				if let Some(id) = reaction_xml_id {
					last_dose_map.get(&id).cloned().unwrap_or((None, None))
				} else if last_dose_map.len() == 1 {
					last_dose_map
						.values()
						.next()
						.cloned()
						.unwrap_or((None, None))
				} else {
					(None, None)
				};
			let rechallenge_action = normalize_code(
				first_attr(
					&mut xpath,
					&obs,
					"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.1']]/hl7:value",
					"code",
				),
				&["1", "2", "3", "4"],
				"drug_recurrence_information.rechallenge_action",
			);
			let recurrence_meddra_version = clamp_str(
				first_attr(
					&mut xpath,
					&obs,
					"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.2']]/hl7:value",
					"codeSystemVersion",
				),
				10,
				"drug_recurrence_information.reaction_meddra_version",
			);
			let recurrence_meddra_code = first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.2']]/hl7:value",
				"code",
			);
			observations.push(DrugObservationImport {
				drug_xml_id,
				drug_sequence,
				sequence_number,
				reaction_xml_id,
				administration_start_interval_value,
				administration_start_interval_unit,
				last_dose_interval_value,
				last_dose_interval_unit,
				reaction_recurred,
				rechallenge_action,
				recurrence_meddra_version,
				recurrence_meddra_code,
			});
		}
	}

	Ok(observations)
}

pub(crate) fn parse_relatedness_assessments(
	xml: &[u8],
) -> Result<Vec<RelatednessImport>> {
	let (_doc, mut xpath) = build_xpath(xml)?;
	let nodes = xpath
		.findnodes(
			"//hl7:component[hl7:causalityAssessment/hl7:code[@code='39']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query relatedness assessments".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let source_of_assessment = first_text(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:author/hl7:assignedEntity/hl7:code/hl7:originalText",
		);
		let method_of_assessment = first_text(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:methodCode/hl7:originalText",
		)
		.or_else(|| {
			first_attr(
				&mut xpath,
				&node,
				"hl7:causalityAssessment/hl7:methodCode",
				"code",
			)
		});
		let result_of_assessment =
			first_text(&mut xpath, &node, "hl7:causalityAssessment/hl7:value")
				.or_else(|| {
					first_attr(
						&mut xpath,
						&node,
						"hl7:causalityAssessment/hl7:value",
						"code",
					)
				});
		let reaction_xml_id = parse_uuid_opt(first_attr(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:subject1/hl7:adverseEffectReference/hl7:id",
			"root",
		));
		let drug_xml_id = parse_uuid_opt(first_attr(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:subject2/hl7:productUseReference/hl7:id",
			"root",
		));

		items.push(RelatednessImport {
			drug_xml_id,
			reaction_xml_id,
			source_of_assessment,
			method_of_assessment,
			result_of_assessment,
		});
	}

	Ok(items)
}
