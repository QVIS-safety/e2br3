// Section G importer (Drug/Biological) - FDA mapping.

use crate::xml::error::Error;
use crate::xml::mapping::fda::g_drug::GDrugPaths;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Node;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use sqlx::types::Uuid;

#[derive(Debug)]
pub struct GDrugImport {
	pub xml_id: Option<Uuid>,
	pub sequence_number: i32,
	pub medicinal_product: String,
	pub brand_name: Option<String>,
	pub drug_characterization: String,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub obtain_drug_country: Option<String>,
	pub manufacturer_name: Option<String>,
	pub manufacturer_country: Option<String>,
	pub batch_lot_number: Option<String>,
	pub dosage_text: Option<String>,
	pub action_taken: Option<String>,
	pub rechallenge: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub parent_dosage_text: Option<String>,
	pub fda_additional_info_coded: Option<String>,
	pub substances: Vec<GDrugSubstanceImport>,
	pub dosages: Vec<GDrugDosageImport>,
	pub indications: Vec<GDrugIndicationImport>,
	pub characteristics: Vec<GDrugDeviceCharacteristicImport>,
}

#[derive(Debug)]
pub struct GDrugSubstanceImport {
	pub substance_name: Option<String>,
	pub substance_termid: Option<String>,
	pub substance_termid_version: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

#[derive(Debug)]
pub struct GDrugDosageImport {
	pub dosage_text: Option<String>,
	pub frequency_value: Option<Decimal>,
	pub frequency_unit: Option<String>,
	pub start_date: Option<sqlx::types::time::Date>,
	pub end_date: Option<sqlx::types::time::Date>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub dose_value: Option<Decimal>,
	pub dose_unit: Option<String>,
	pub route: Option<String>,
	pub dose_form: Option<String>,
	pub dose_form_termid: Option<String>,
	pub dose_form_termid_version: Option<String>,
	pub batch_lot: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub parent_route: Option<String>,
}

#[derive(Debug)]
pub struct GDrugIndicationImport {
	pub text: Option<String>,
	pub version: Option<String>,
	pub code: Option<String>,
}

#[derive(Debug)]
pub struct GDrugDeviceCharacteristicImport {
	pub code: Option<String>,
	pub code_system: Option<String>,
	pub code_display_name: Option<String>,
	pub value_type: Option<String>,
	pub value_value: Option<String>,
	pub value_code: Option<String>,
	pub value_code_system: Option<String>,
	pub value_display_name: Option<String>,
}

pub fn parse_g_drugs(xml: &[u8]) -> Result<Vec<GDrugImport>> {
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
	let _ =
		xpath.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");

	let drug_nodes = xpath.findnodes(GDrugPaths::DRUG_NODE, None).map_err(|_| {
		Error::InvalidXml {
			message: "Failed to query drug information".to_string(),
			line: None,
			column: None,
		}
	})?;

	let mut imports: Vec<GDrugImport> = Vec::new();
	for (idx, node) in drug_nodes.into_iter().enumerate() {
		let xml_id =
			parse_uuid_opt(first_attr(&mut xpath, &node, GDrugPaths::XML_ID_ROOT));
		let name1 = first_text(&mut xpath, &node, GDrugPaths::PRODUCT_NAME_1)
			.ok_or_else(|| Error::InvalidXml {
				message: format!(
					"ICH.G.k.2.2.REQUIRED: medicinal product name missing for drug index {}",
					idx + 1
				),
				line: None,
				column: None,
			})?;
		let name2 = first_text(&mut xpath, &node, GDrugPaths::PRODUCT_NAME_2);
		let drug_characterization = "1".to_string();
		let mpid = first_attr(&mut xpath, &node, GDrugPaths::MPID);
		let mpid_version =
			clamp_str(first_attr(&mut xpath, &node, GDrugPaths::MPID_VERSION), 10);
		let investigational_product_blinded =
			first_attr(&mut xpath, &node, GDrugPaths::INVESTIGATIONAL_BLINDED)
				.and_then(parse_bool);
		let manufacturer_name =
			first_text(&mut xpath, &node, GDrugPaths::MANUFACTURER_NAME);
		let manufacturer_country = normalize_iso2(first_attr(
			&mut xpath,
			&node,
			GDrugPaths::MANUFACTURER_COUNTRY,
		));
		let obtain_drug_country = normalize_iso2(first_text(
			&mut xpath,
			&node,
			GDrugPaths::OBTAIN_DRUG_COUNTRY,
		));
		let action_taken = normalize_code(
			first_attr(&mut xpath, &node, GDrugPaths::ACTION_TAKEN),
			&["1", "2", "3", "4", "5", "6"],
		);
		let rechallenge = normalize_code(
			first_attr(&mut xpath, &node, GDrugPaths::RECHALLENGE),
			&["1", "2", "3", "4"],
		);
		let dosage_text = first_text(&mut xpath, &node, GDrugPaths::DOSAGE_TEXT);
		let batch_lot_number =
			first_text(&mut xpath, &node, GDrugPaths::BATCH_LOT_NUMBER);
		let fda_additional_info_coded = clamp_str(
			first_attr(&mut xpath, &node, GDrugPaths::FDA_ADDITIONAL_INFO),
			10,
		);
		let parent_route_termid_version = clamp_str(
			first_attr(&mut xpath, &node, GDrugPaths::PARENT_ROUTE_TERMID_VERSION),
			10,
		);
		let parent_route_termid =
			first_attr(&mut xpath, &node, GDrugPaths::PARENT_ROUTE_TERMID);
		let parent_dosage_text =
			first_text(&mut xpath, &node, GDrugPaths::PARENT_DOSAGE_TEXT);
		let parent_route =
			first_text(&mut xpath, &node, GDrugPaths::PARENT_ROUTE_TEXT);

		let subs = xpath
			.findnodes(GDrugPaths::SUBSTANCE_NODE, Some(&node))
			.unwrap_or_default();
		let mut substances = Vec::new();
		for sub in subs.into_iter() {
			let sub_name = first_text(&mut xpath, &sub, GDrugPaths::SUBSTANCE_NAME);
			let termid = first_attr(&mut xpath, &sub, GDrugPaths::SUBSTANCE_TERMID);
			let termid_version = clamp_str(
				first_attr(&mut xpath, &sub, GDrugPaths::SUBSTANCE_TERMID_VERSION),
				10,
			);
			let strength_value =
				first_attr(&mut xpath, &sub, GDrugPaths::SUBSTANCE_STRENGTH_VALUE)
					.and_then(|v| v.parse::<Decimal>().ok());
			let strength_unit =
				first_attr(&mut xpath, &sub, GDrugPaths::SUBSTANCE_STRENGTH_UNIT);
			substances.push(GDrugSubstanceImport {
				substance_name: sub_name,
				substance_termid: termid,
				substance_termid_version: termid_version,
				strength_value,
				strength_unit,
			});
		}

		let dosages = xpath
			.findnodes(GDrugPaths::DOSAGE_NODE, Some(&node))
			.unwrap_or_default();
		let mut dosage_list = Vec::new();
		for dose in dosages.into_iter() {
			let dosage_text =
				first_text(&mut xpath, &dose, GDrugPaths::DOSAGE_TEXT_NODE);
			let frequency_value =
				first_attr(&mut xpath, &dose, GDrugPaths::DOSAGE_FREQUENCY_VALUE)
					.and_then(|v| v.parse::<Decimal>().ok());
			let frequency_unit = normalize_code3(first_attr(
				&mut xpath,
				&dose,
				GDrugPaths::DOSAGE_FREQUENCY_UNIT,
			));
			let start_date =
				first_attr(&mut xpath, &dose, GDrugPaths::DOSAGE_START_DATE)
					.and_then(parse_date);
			let end_date =
				first_attr(&mut xpath, &dose, GDrugPaths::DOSAGE_END_DATE)
					.and_then(parse_date);
			let duration_value =
				first_attr(&mut xpath, &dose, GDrugPaths::DOSAGE_DURATION_VALUE)
					.and_then(|v| v.parse::<Decimal>().ok());
			let duration_unit = normalize_code3(first_attr(
				&mut xpath,
				&dose,
				GDrugPaths::DOSAGE_DURATION_UNIT,
			));
			let dose_value = first_attr(&mut xpath, &dose, GDrugPaths::DOSE_VALUE)
				.and_then(|v| v.parse::<Decimal>().ok());
			let dose_unit = first_attr(&mut xpath, &dose, GDrugPaths::DOSE_UNIT);
			let route = normalize_code3(first_attr(
				&mut xpath,
				&dose,
				GDrugPaths::ROUTE_CODE,
			));
			let dose_form =
				first_text(&mut xpath, &dose, GDrugPaths::DOSE_FORM_TEXT);
			let dose_form_termid =
				first_attr(&mut xpath, &dose, GDrugPaths::DOSE_FORM_TERMID);
			let dose_form_termid_version = clamp_str(
				first_attr(&mut xpath, &dose, GDrugPaths::DOSE_FORM_TERMID_VERSION),
				10,
			);
			let batch_lot =
				first_text(&mut xpath, &dose, GDrugPaths::DOSAGE_BATCH_LOT);
			let parent_route_termid = first_attr(
				&mut xpath,
				&dose,
				GDrugPaths::DOSAGE_PARENT_ROUTE_TERMID,
			);
			let parent_route_termid_version = clamp_str(
				first_attr(
					&mut xpath,
					&dose,
					GDrugPaths::DOSAGE_PARENT_ROUTE_TERMID_VERSION,
				),
				10,
			);
			let parent_route =
				first_text(&mut xpath, &dose, GDrugPaths::DOSAGE_PARENT_ROUTE_TEXT);

			dosage_list.push(GDrugDosageImport {
				dosage_text,
				frequency_value,
				frequency_unit,
				start_date,
				end_date,
				duration_value,
				duration_unit,
				dose_value,
				dose_unit,
				route,
				dose_form,
				dose_form_termid,
				dose_form_termid_version,
				batch_lot,
				parent_route_termid,
				parent_route_termid_version,
				parent_route,
			});
		}

		let inds = xpath
			.findnodes(GDrugPaths::INDICATION_NODE, Some(&node))
			.unwrap_or_default();
		let mut indications = Vec::new();
		for ind in inds.into_iter() {
			let text = first_text(&mut xpath, &ind, GDrugPaths::INDICATION_TEXT);
			let code = first_attr(&mut xpath, &ind, GDrugPaths::INDICATION_CODE);
			let version = clamp_str(
				first_attr(&mut xpath, &ind, GDrugPaths::INDICATION_VERSION),
				10,
			);
			indications.push(GDrugIndicationImport {
				text,
				version,
				code,
			});
		}

		let chars = xpath
			.findnodes(GDrugPaths::DEVICE_CHAR_NODE, Some(&node))
			.unwrap_or_default();
		let mut characteristics = Vec::new();
		for ch in chars.into_iter() {
			let code = first_attr(&mut xpath, &ch, GDrugPaths::DEVICE_CHAR_CODE);
			let code_system =
				first_attr(&mut xpath, &ch, GDrugPaths::DEVICE_CHAR_CODE_SYSTEM);
			let code_display_name =
				first_attr(&mut xpath, &ch, GDrugPaths::DEVICE_CHAR_DISPLAY);
			let value_type = clamp_str(
				first_attr(&mut xpath, &ch, GDrugPaths::DEVICE_CHAR_VALUE_TYPE)
					.or_else(|| {
						first_attr(
							&mut xpath,
							&ch,
							GDrugPaths::DEVICE_CHAR_VALUE_TYPE_ALT,
						)
					}),
				10,
			);
			let value_value =
				first_attr(&mut xpath, &ch, GDrugPaths::DEVICE_CHAR_VALUE_VALUE);
			let value_code =
				first_attr(&mut xpath, &ch, GDrugPaths::DEVICE_CHAR_VALUE_CODE);
			let value_code_system = first_attr(
				&mut xpath,
				&ch,
				GDrugPaths::DEVICE_CHAR_VALUE_CODE_SYSTEM,
			);
			let value_display_name =
				first_attr(&mut xpath, &ch, GDrugPaths::DEVICE_CHAR_VALUE_DISPLAY);
			characteristics.push(GDrugDeviceCharacteristicImport {
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

		imports.push(GDrugImport {
			xml_id,
			sequence_number: (idx + 1) as i32,
			medicinal_product: name1,
			brand_name: name2,
			drug_characterization,
			mpid,
			mpid_version,
			investigational_product_blinded,
			obtain_drug_country,
			manufacturer_name,
			manufacturer_country,
			batch_lot_number,
			dosage_text,
			action_taken,
			rechallenge,
			parent_route,
			parent_route_termid,
			parent_route_termid_version,
			parent_dosage_text,
			fda_additional_info_coded,
			substances,
			dosages: dosage_list,
			indications,
			characteristics,
		});
	}

	Ok(imports)
}

fn first_attr(xpath: &mut Context, node: &Node, expr: &str) -> Option<String> {
	xpath
		.findvalues(expr, Some(node))
		.ok()?
		.into_iter()
		.find(|v| !v.trim().is_empty())
}

fn first_text(xpath: &mut Context, node: &Node, expr: &str) -> Option<String> {
	let nodes = xpath.findnodes(expr, Some(node)).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

fn parse_bool(value: String) -> Option<bool> {
	match value.to_ascii_lowercase().as_str() {
		"true" | "1" => Some(true),
		"false" | "0" => Some(false),
		_ => None,
	}
}

fn clamp_str(value: Option<String>, max: usize) -> Option<String> {
	match value {
		Some(v) if v.len() > max => Some(v.chars().take(max).collect()),
		other => other,
	}
}

fn parse_uuid_opt(value: Option<String>) -> Option<Uuid> {
	let value = value?.trim().to_string();
	if value.is_empty() {
		return None;
	}
	Uuid::parse_str(&value).ok()
}

fn normalize_iso2(value: Option<String>) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	let upper = v.to_ascii_uppercase();
	if len == 2 && upper.chars().all(|c| c.is_ascii_uppercase()) {
		Some(upper)
	} else {
		None
	}
}

fn normalize_code(value: Option<String>, allowed: &[&str]) -> Option<String> {
	let candidate = value?;
	if allowed.iter().any(|v| *v == candidate) {
		Some(candidate)
	} else {
		None
	}
}

fn normalize_code3(value: Option<String>) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	if (1..=3).contains(&len) && v.chars().all(|c| c.is_ascii_alphanumeric()) {
		Some(v)
	} else {
		None
	}
}

fn parse_date(value: String) -> Option<sqlx::types::time::Date> {
	let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
	if digits.len() < 8 {
		return None;
	}
	let y: i32 = digits[0..4].parse().ok()?;
	let m: u8 = digits[4..6].parse().ok()?;
	let d: u8 = digits[6..8].parse().ok()?;
	let month = time::Month::try_from(m).ok()?;
	sqlx::types::time::Date::from_calendar_date(y, month, d).ok()
}
