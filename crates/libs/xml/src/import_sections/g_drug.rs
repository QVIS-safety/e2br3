use crate::import_sections::shared::first_text;
// Section G importer (Drug/Biological) - FDA mapping.

use crate::error::Error;
use crate::import_constraint::Context;
use crate::import_sections::shared::parse_bool_literal;
use crate::mapping::fda::g_drug::GDrugPaths;
use crate::mapping::mfds::g_drug::GMfdsDrugPaths;
use crate::Result;
use base64::Engine as _;
use libxml::tree::Node;
use rust_decimal::Decimal;
use std::collections::HashMap;

mod helpers;
pub mod product_links;
mod runtime;
pub(crate) use runtime::import_section_g;

#[derive(Debug)]
pub struct GDrugImport {
	pub xml_id: Option<String>,
	pub source_product_link: product_links::SourceProductLink,
	pub source_product_id: Option<String>,
	pub sequence_number: i32,
	pub medicinal_product: String,
	pub drug_characterization: String,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mpid_source_code_system: Option<String>,
	pub mpid_source_code_system_version: Option<String>,
	pub mfds_mpid: Option<String>,
	pub mfds_mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub fda_other_characterization: Option<String>,
	pub obtain_drug_country: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub manufacturer_name: Option<String>,
	pub manufacturer_country: Option<String>,
	pub batch_lot_number: Option<String>,
	pub cumulative_dose_first_reaction_value: Option<Decimal>,
	pub cumulative_dose_first_reaction_unit: Option<String>,
	pub gestation_period_exposure_value: Option<Decimal>,
	pub gestation_period_exposure_unit: Option<String>,
	pub action_taken: Option<String>,
	pub fda_additional_info_coded: Option<String>,
	pub fda_additional_info_coded_null_flavor: Option<String>,
	pub fda_specialized_product_category: Option<String>,
	pub drug_additional_information: Option<String>,
	pub drug_additional_info_codes_json: Option<serde_json::Value>,
	pub devices: Vec<GDrugFdaDeviceImport>,
	pub substances: Vec<GDrugSubstanceImport>,
	pub dosages: Vec<GDrugDosageImport>,
	pub indications: Vec<GDrugIndicationImport>,
	pub characteristics: Vec<GDrugDeviceCharacteristicImport>,
}

#[derive(Debug)]
pub struct GDrugFdaDeviceImport {
	pub malfunction: Option<bool>,
	pub device_brand_name: Option<String>,
	pub device_brand_name_null_flavor: Option<String>,
	pub common_device_name: Option<String>,
	pub common_device_name_null_flavor: Option<String>,
	pub device_product_code: Option<String>,
	pub manufacturer_name: Option<String>,
	pub manufacturer_address: Option<String>,
	pub manufacturer_city: Option<String>,
	pub manufacturer_state: Option<String>,
	pub manufacturer_country: Option<String>,
	pub device_usage: Option<String>,
	pub device_lot_number: Option<String>,
	pub operator_of_device: Option<String>,
	pub codes: Vec<GDrugFdaDeviceCodeImport>,
}

#[derive(Debug)]
pub struct GDrugFdaDeviceCodeImport {
	pub element: &'static str,
	pub value_code: String,
}

#[derive(Debug)]
pub struct GDrugSubstanceImport {
	pub substance_name: Option<String>,
	pub substance_termid: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid_code_system: Option<String>,
	pub mfds_id: Option<String>,
	pub mfds_version: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

#[derive(Debug)]
pub struct GDrugDosageImport {
	pub continuing: Option<bool>,
	pub dosage_text: Option<String>,
	pub frequency_unit: Option<String>,
	pub number_of_units: Option<Decimal>,
	pub start_date: Option<sqlx::types::time::Date>,
	pub start_date_raw: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub end_date: Option<sqlx::types::time::Date>,
	pub end_date_raw: Option<String>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub dose_value: Option<Decimal>,
	pub dose_unit: Option<String>,
	pub route: Option<String>,
	pub route_null_flavor: Option<String>,
	pub route_termid: Option<String>,
	pub route_termid_version: Option<String>,
	pub route_termid_code_system: Option<String>,
	pub dose_form: Option<String>,
	pub dose_form_null_flavor: Option<String>,
	pub dose_form_termid: Option<String>,
	pub dose_form_termid_version: Option<String>,
	pub batch_lot: Option<String>,
	pub batch_lot_null_flavor: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub parent_route_termid_code_system: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_null_flavor: Option<String>,
}

#[derive(Debug)]
pub struct GDrugIndicationImport {
	pub text: Option<String>,
	pub text_null_flavor: Option<String>,
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
	let doc = crate::import_sections::shared::parse_import_xml(xml_str)?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let result = (|| -> Result<_> {
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		let _ = xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");

		let drug_nodes =
			xpath.findnodes(GDrugPaths::DRUG_NODE, None).map_err(|_| {
				Error::InvalidXml {
					message: "Failed to query drug information".to_string(),
					line: None,
					column: None,
				}
			})?;
		let drug_roles = read_drug_roles(&mut xpath)?;
		let fda_other_roles = read_fda_other_characterizations(&mut xpath)?;

		let mut imports: Vec<GDrugImport> = Vec::new();
		for (idx, node) in drug_nodes.into_iter().enumerate() {
			let xml_id = read_xml_id(&mut xpath, &node, idx + 1)?;
			let name1 = read_g_k_2_2(&mut xpath, &node, idx + 1)?;
			let drug_characterization =
				read_g_k_1(&drug_roles, &xml_id, idx + 1, &mut xpath)?;
			let (
				mpid,
				mpid_version,
				mpid_source_code_system,
				mpid_source_code_system_version,
			) = read_mpid(&mut xpath, &node)?;
			let mfds_mpid = read_g_k_2_1_kr_1b(&mut xpath, &node)?;
			let mfds_mpid_version = read_g_k_2_1_kr_1a(&mut xpath, &node)?;
			let phpid = read_g_k_2_1_2a(&mut xpath, &node)?;
			let phpid_version = read_g_k_2_1_2b(&mut xpath, &node)?;
			let investigational_product_blinded = read_g_k_2_5(&mut xpath, &node)?;
			let fda_other_characterization = fda_other_roles.get(&xml_id).cloned();
			let drug_authorization_number = read_g_k_3_1(&mut xpath, &node)?;
			let manufacturer_name = read_g_k_3_2(&mut xpath, &node)?;
			let manufacturer_country = read_g_k_3_3(&mut xpath, &node)?;
			let obtain_drug_country = read_g_k_2_4(&mut xpath, &node)?;
			let action_taken = read_g_k_8(&mut xpath, &node)?;
			let batch_lot_number = read_g_k_local_batch_lot(&mut xpath, &node);
			let cumulative_dose_first_reaction_value =
				read_g_k_5a(&mut xpath, &node)?;
			let cumulative_dose_first_reaction_unit =
				read_g_k_5b(&mut xpath, &node)?;
			let gestation_period_exposure_value = read_g_k_6a(&mut xpath, &node)?;
			let gestation_period_exposure_unit = read_g_k_6b(&mut xpath, &node)?;
			let (fda_additional_info_coded, fda_additional_info_coded_null_flavor) =
				read_fda_g_k_10_1(&mut xpath, &node)?;
			let fda_specialized_product_category =
				read_fda_g_k_10a(&mut xpath, &node)?;
			let drug_additional_information = read_g_k_11(&mut xpath, &node)?;
			let drug_additional_info_codes_json = read_g_k_10_r(&mut xpath, &node)?;
			let mut devices = Vec::new();
			for device in
				find_nodes(&mut xpath, GDrugPaths::DEVICE_NODE, Some(&node))?
			{
				let mut codes = Vec::new();
				for characteristic in find_nodes(
					&mut xpath,
					GDrugPaths::DEVICE_CHARACTERISTIC_NODE,
					Some(&device),
				)? {
					for (element, value_code) in [
						(
							"follow_up_type",
							read_fda_g_k_12_r_2_r(&mut xpath, &characteristic)?,
						),
						(
							"device_problem",
							read_fda_g_k_12_r_3_r(&mut xpath, &characteristic)?,
						),
						(
							"remedial_action",
							read_fda_g_k_12_r_11_r(&mut xpath, &characteristic)?,
						),
					] {
						if let Some(value_code) = value_code {
							codes.push(GDrugFdaDeviceCodeImport {
								element,
								value_code,
							});
						}
					}
				}
				let malfunction = read_fda_g_k_12_r_1(&mut xpath, &device)?;
				devices.push(GDrugFdaDeviceImport {
					malfunction,
					device_brand_name: read_fda_g_k_12_r_4(&mut xpath, &device)?,
					device_brand_name_null_flavor: first_attr(
						&mut xpath,
						&device,
						"hl7:partProduct/hl7:name[1]/@nullFlavor",
					),
					common_device_name: read_fda_g_k_12_r_5(&mut xpath, &device)?,
					common_device_name_null_flavor: first_attr(
						&mut xpath,
						&device,
						"hl7:partProduct/hl7:name[2]/@nullFlavor",
					),
					device_product_code: read_fda_g_k_12_r_6(&mut xpath, &device)?,
					manufacturer_name: read_fda_g_k_12_r_7_1a(&mut xpath, &device)?,
					manufacturer_address: read_fda_g_k_12_r_7_1b(
						&mut xpath, &device,
					)?,
					manufacturer_city: read_fda_g_k_12_r_7_1c(&mut xpath, &device)?,
					manufacturer_state: read_fda_g_k_12_r_7_1d(&mut xpath, &device)?,
					manufacturer_country: read_fda_g_k_12_r_7_1e(
						&mut xpath, &device,
					)?,
					device_usage: read_fda_g_k_12_r_8(&mut xpath, &device)?,
					device_lot_number: read_fda_g_k_12_r_9(&mut xpath, &device)?,
					operator_of_device: read_fda_g_k_12_r_10(&mut xpath, &device)?,
					codes,
				});
			}
			let subs =
				find_nodes(&mut xpath, GDrugPaths::SUBSTANCE_NODE, Some(&node))?;
			let mut substances = Vec::new();
			for sub in subs.into_iter() {
				let sub_name = read_g_k_2_3_r_1(&mut xpath, &sub)?;
				let termid = read_g_k_2_3_r_2b(&mut xpath, &sub)?;
				let termid_version = read_g_k_2_3_r_2a(&mut xpath, &sub)?;
				let termid_code_system =
					read_g_k_2_3_r_2_code_system(&mut xpath, &sub);
				let mfds_id = read_g_k_2_3_r_1_kr_1b(&mut xpath, &sub)?;
				let mfds_version = read_g_k_2_3_r_1_kr_1a(&mut xpath, &sub)?;
				let strength_value = read_g_k_2_3_r_3a(&mut xpath, &sub)?;
				let strength_unit = read_g_k_2_3_r_3b(&mut xpath, &sub)?;
				substances.push(GDrugSubstanceImport {
					substance_name: sub_name,
					substance_termid: termid,
					substance_termid_version: termid_version,
					substance_termid_code_system: termid_code_system,
					mfds_id,
					mfds_version,
					strength_value,
					strength_unit,
				});
			}

			let dosages =
				find_nodes(&mut xpath, GDrugPaths::DOSAGE_NODE, Some(&node))?;
			let mut dosage_list = Vec::new();
			for dose in dosages.into_iter() {
				let dosage_text = read_g_k_4_r_8(&mut xpath, &dose)?;
				let frequency_unit = read_g_k_4_r_3(&mut xpath, &dose)?;
				let number_of_units = read_g_k_4_r_2(&mut xpath, &dose)?;
				let (start_date, start_date_raw, start_date_null_flavor) =
					read_g_k_4_r_4(&mut xpath, &dose)?;
				let (end_date, end_date_raw, end_date_null_flavor) =
					read_g_k_4_r_5(&mut xpath, &dose)?;
				let duration_value = read_g_k_4_r_6a(&mut xpath, &dose)?;
				let duration_unit = read_g_k_4_r_6b(&mut xpath, &dose)?;
				let dose_value = read_g_k_4_r_1a(&mut xpath, &dose)?;
				let dose_unit = read_g_k_4_r_1b(&mut xpath, &dose)?;
				let (route, route_null_flavor) =
					read_g_k_4_r_10_1(&mut xpath, &dose)?;
				let route_termid = read_g_k_4_r_10_2b(&mut xpath, &dose)?;
				let route_termid_version = read_g_k_4_r_10_2a(&mut xpath, &dose)?;
				let route_termid_code_system =
					read_g_k_4_r_10_2_code_system(&mut xpath, &dose)?;
				let (dose_form, dose_form_null_flavor) =
					read_g_k_4_r_9_1(&mut xpath, &dose)?;
				let dose_form_termid = read_g_k_4_r_9_2b(&mut xpath, &dose)?;
				let dose_form_termid_version = read_g_k_4_r_9_2a(&mut xpath, &dose)?;
				let (batch_lot, batch_lot_null_flavor) =
					read_g_k_4_r_7(&mut xpath, &dose)?;
				let parent_route_termid = read_g_k_4_r_11_2b(&mut xpath, &dose)?;
				let parent_route_termid_version =
					read_g_k_4_r_11_2a(&mut xpath, &dose)?;
				let parent_route_termid_code_system =
					read_g_k_4_r_11_2_code_system(&mut xpath, &dose)?;
				let (parent_route, parent_route_null_flavor) =
					read_g_k_4_r_11_1(&mut xpath, &dose)?;

				dosage_list.push(GDrugDosageImport {
					continuing: read_local_continuing(&mut xpath, &dose)?,
					dosage_text,
					frequency_unit,
					number_of_units,
					start_date,
					start_date_raw,
					start_date_null_flavor,
					end_date,
					end_date_raw,
					end_date_null_flavor,
					duration_value,
					duration_unit,
					dose_value,
					dose_unit,
					route,
					route_null_flavor,
					route_termid,
					route_termid_version,
					route_termid_code_system,
					dose_form,
					dose_form_null_flavor,
					dose_form_termid,
					dose_form_termid_version,
					batch_lot,
					batch_lot_null_flavor,
					parent_route_termid,
					parent_route_termid_version,
					parent_route_termid_code_system,
					parent_route,
					parent_route_null_flavor,
				});
			}

			let inds =
				find_nodes(&mut xpath, GDrugPaths::INDICATION_NODE, Some(&node))?;
			let mut indications = Vec::new();
			for ind in inds.into_iter() {
				let (text, text_null_flavor) = read_g_k_7_r_1(&mut xpath, &ind)?;
				let code = read_g_k_7_r_2b(&mut xpath, &ind)?;
				let version = read_g_k_7_r_2a(&mut xpath, &ind)?;
				indications.push(GDrugIndicationImport {
					text,
					text_null_flavor,
					version,
					code,
				});
			}

			let chars =
				find_nodes(&mut xpath, GDrugPaths::DEVICE_CHAR_NODE, Some(&node))?;
			let mut characteristics = Vec::new();
			for ch in chars.into_iter() {
				let code = read_device_characteristic_code(&mut xpath, &ch);
				let code_system =
					read_device_characteristic_code_system(&mut xpath, &ch);
				let code_display_name =
					read_device_characteristic_code_display_name(&mut xpath, &ch);
				// MFDS device fields are UI-only; do not import retired vendor extensions.
				if [code.as_deref(), code_display_name.as_deref()]
					.into_iter()
					.flatten()
					.any(|value| {
						value
							.chars()
							.filter(char::is_ascii_alphanumeric)
							.collect::<String>()
							.to_ascii_uppercase()
							.starts_with("KRDVC")
					}) {
					continue;
				}
				let value_type =
					read_device_characteristic_value_type(&mut xpath, &ch);
				let value_value =
					read_device_characteristic_value_value(&mut xpath, &ch);
				let value_code =
					read_device_characteristic_value_code(&mut xpath, &ch);
				let value_code_system =
					read_device_characteristic_value_code_system(&mut xpath, &ch);
				let value_display_name =
					read_device_characteristic_value_display_name(&mut xpath, &ch);
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
				xml_id: Some(xml_id),
				source_product_link: read_local_product_link(&mut xpath, &node)?,
				source_product_id: read_local_source_product_id(&mut xpath, &node)?,
				sequence_number: (idx + 1) as i32,
				medicinal_product: name1,
				drug_characterization,
				mpid,
				mpid_version,
				mpid_source_code_system,
				mpid_source_code_system_version,
				mfds_mpid,
				mfds_mpid_version,
				phpid,
				phpid_version,
				investigational_product_blinded,
				fda_other_characterization,
				obtain_drug_country,
				drug_authorization_number,
				manufacturer_name,
				manufacturer_country,
				batch_lot_number,
				cumulative_dose_first_reaction_value,
				cumulative_dose_first_reaction_unit,
				gestation_period_exposure_value,
				gestation_period_exposure_unit,
				action_taken,
				fda_additional_info_coded,
				fda_additional_info_coded_null_flavor,
				fda_specialized_product_category,
				drug_additional_information,
				drug_additional_info_codes_json,
				devices,
				substances,
				dosages: dosage_list,
				indications,
				characteristics,
			});
		}

		let local_markers = find_nodes(&mut xpath, "//processing-instruction()[starts-with(name(), 'e2br3-dosage-continuing')]", None)?;
		let imported_markers = imports
			.iter()
			.flat_map(|drug| &drug.dosages)
			.filter(|dose| dose.continuing.is_some())
			.count();
		if local_markers.len() != imported_markers {
			return Err(invalid_field(
				"G.k.local.dosage.continuing",
				"Local metadata must belong to an imported dosage row",
			));
		}
		let product_markers = find_nodes(&mut xpath, "//processing-instruction()[starts-with(name(), 'e2br3-source-product-id')]", None)?;
		if product_markers.len()
			!= imports
				.iter()
				.filter(|drug| drug.source_product_id.is_some())
				.count()
		{
			return Err(invalid_field(
				"G.k.local.sourceProductId",
				"Local metadata must belong to an imported drug row",
			));
		}
		let link_markers = find_nodes(
			&mut xpath,
			"//processing-instruction()[starts-with(name(), 'e2br3-product-link')]",
			None,
		)?;
		let local_links = imports
			.iter()
			.filter(|drug| {
				drug.source_product_link != product_links::SourceProductLink::Legacy
			})
			.count();
		if link_markers.len() != local_links
			|| (local_links != 0 && local_links != imports.len())
		{
			return Err(invalid_field("G.k.local.sourceProductPresaveId", "Product link metadata must belong to, and cover, every imported drug row"));
		}
		Ok(imports)
	})()?;
	xpath.validate()?;
	Ok(result)
}

fn read_xml_id(xpath: &mut Context, node: &Node, index: usize) -> Result<String> {
	first_attr(xpath, node, GDrugPaths::XML_ID_ROOT).ok_or_else(|| {
		Error::InvalidImportRequest {
			message: format!("drug {index} is missing XML ID"),
		}
	})
}

fn read_local_product_link(
	xpath: &mut Context,
	drug: &Node,
) -> Result<product_links::SourceProductLink> {
	use product_links::SourceProductLink;
	let markers = find_nodes(
		xpath,
		"processing-instruction()[starts-with(name(), 'e2br3-product-link')]",
		Some(drug),
	)?;
	if markers.is_empty() {
		return Ok(SourceProductLink::Legacy);
	}
	if markers.len() != 1 || markers[0].get_name() != "e2br3-product-link-v1" {
		return Err(invalid_field(
			"G.k.local.sourceProductPresaveId",
			"Unsupported or duplicate product link metadata",
		));
	}
	let value = markers[0].get_content();
	if value == "none" {
		return Ok(SourceProductLink::Unlinked);
	}
	let id = uuid::Uuid::parse_str(&value).map_err(|_| {
		invalid_field(
			"G.k.local.sourceProductPresaveId",
			"Product link metadata requires a canonical non-nil UUID or none",
		)
	})?;
	if id.is_nil() || id.to_string() != value {
		return Err(invalid_field(
			"G.k.local.sourceProductPresaveId",
			"Product link metadata requires a canonical non-nil UUID or none",
		));
	}
	Ok(SourceProductLink::Linked(id))
}

fn read_local_source_product_id(
	xpath: &mut Context,
	drug: &Node,
) -> Result<Option<String>> {
	let markers = find_nodes(
		xpath,
		"processing-instruction()[starts-with(name(), 'e2br3-source-product-id')]",
		Some(drug),
	)?;
	if markers.is_empty() {
		return Ok(None);
	}
	if markers.len() != 1 || markers[0].get_name() != "e2br3-source-product-id-v1" {
		return Err(invalid_field(
			"G.k.local.sourceProductId",
			"Unsupported or duplicate local metadata",
		));
	}
	let bytes = base64::engine::general_purpose::STANDARD
		.decode(markers[0].get_content())
		.map_err(|_| {
			invalid_field(
				"G.k.local.sourceProductId",
				"Local metadata must be valid base64",
			)
		})?;
	let value = String::from_utf8(bytes).map_err(|_| {
		invalid_field(
			"G.k.local.sourceProductId",
			"Local metadata must encode UTF-8",
		)
	})?;
	if value.contains('\0') || value.chars().count() > 255 {
		return Err(invalid_field(
			"G.k.local.sourceProductId",
			"Local metadata must contain at most 255 characters and no NUL",
		));
	}
	Ok(Some(value))
}

fn read_local_continuing(xpath: &mut Context, dose: &Node) -> Result<Option<bool>> {
	let markers = find_nodes(
		xpath,
		"processing-instruction()[starts-with(name(), 'e2br3-dosage-continuing')]",
		Some(dose),
	)?;
	if markers.is_empty() {
		return Ok(None);
	}
	if markers.len() != 1 || markers[0].get_name() != "e2br3-dosage-continuing-v1" {
		return Err(invalid_field(
			"G.k.local.dosage.continuing",
			"Unsupported or duplicate local metadata",
		));
	}
	match markers[0].get_content().trim() {
		"true" => Ok(Some(true)),
		"false" => Ok(Some(false)),
		_ => Err(invalid_field(
			"G.k.local.dosage.continuing",
			"Local metadata requires literal true or false",
		)),
	}
}

/// e2b:G.k.1
fn read_g_k_1(
	drug_roles: &HashMap<String, String>,
	xml_id: &str,
	_index: usize,
	xpath: &mut Context,
) -> Result<String> {
	let value = drug_roles.get(xml_id).ok_or_else(|| {
		invalid_field(
			"G.k.1",
			format!("drug characterization missing for product {xml_id}"),
		)
	})?;
	xpath.inputs.string(
		"drugCharacterization",
		Some(value),
		None,
		input_contracts::generated::g::g_k_1,
	);
	Ok(value.clone())
}

fn read_drug_roles(xpath: &mut Context) -> Result<HashMap<String, String>> {
	let nodes = find_nodes(xpath, GDrugPaths::DRUG_ROLE_NODE, None)?;
	let mut roles = HashMap::new();
	for node in nodes {
		let Some(reference) =
			first_attr(xpath, &node, GDrugPaths::DRUG_ROLE_PRODUCT_REF)
		else {
			continue;
		};
		let value = first_attr(xpath, &node, GDrugPaths::DRUG_ROLE_CODE)
			.ok_or_else(|| Error::InvalidImportRequest {
				message: "drug role code is required".to_string(),
			})?;
		xpath.inputs.string(
			"drugCharacterization",
			Some(&value),
			None,
			input_contracts::generated::g::g_k_1,
		);
		if let Some(previous) = roles.insert(reference.to_string(), value.clone()) {
			if previous != value {
				return Err(invalid_field(
					"G.k.1",
					format!("conflicting drug characterizations for {reference}"),
				));
			}
		}
	}
	Ok(roles)
}

/// e2b:FDA.G.k.1.a
fn read_fda_other_characterizations(
	xpath: &mut Context,
) -> Result<HashMap<String, String>> {
	let nodes = find_nodes(xpath, GDrugPaths::FDA_OTHER_ROLE_NODE, None)?;
	let mut roles = HashMap::new();
	for node in nodes {
		let Some(reference) =
			first_attr(xpath, &node, GDrugPaths::DRUG_ROLE_PRODUCT_REF)
		else {
			continue;
		};
		let Some(value) = first_attr(xpath, &node, GDrugPaths::FDA_OTHER_ROLE_CODE)
		else {
			continue;
		};
		xpath.inputs.string(
			"fdaOtherCharacterization",
			Some(&value),
			None,
			input_contracts::generated::g::fda_g_k_1_a,
		);
		if let Some(previous) = roles.insert(reference.to_string(), value.clone()) {
			if previous != value {
				return Err(invalid_field(
					"FDA.G.k.1.a",
					format!(
						"conflicting FDA drug characterizations for {reference}"
					),
				));
			}
		}
	}
	Ok(roles)
}

/// e2b:G.k.2.2
fn read_g_k_2_2(xpath: &mut Context, node: &Node, _index: usize) -> Result<String> {
	let value =
		first_text(xpath, node, GDrugPaths::PRODUCT_NAME_1).ok_or_else(|| {
			Error::InvalidImportRequest {
				message: "medicinal product name is required".to_string(),
			}
		})?;
	xpath.inputs.string(
		"medicinalProduct",
		Some(&value),
		None,
		input_contracts::generated::g::g_k_2_2,
	);
	Ok(value)
}

/// e2b:G.k.2.1.1a
fn read_g_k_2_1_1a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::MPID);
	xpath.inputs.string(
		"mpid",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_2_1_1b,
	);
	Ok(value)
}

fn read_mpid(
	xpath: &mut Context,
	node: &Node,
) -> Result<(
	Option<String>,
	Option<String>,
	Option<String>,
	Option<String>,
)> {
	if let Some(mpid) = first_attr(xpath, node, GDrugPaths::FDA_MPID) {
		if let Some(identifier) =
			first_attr(xpath, node, GDrugPaths::IDENTIFIED_MPID)
		{
			if identifier != mpid {
				return Err(Error::InvalidImportRequest {
					message:
						"G.k.2.1.1b contains conflicting FDA and MPID identifiers"
							.to_owned(),
				});
			}
		}
		let identified_version =
			first_attr(xpath, node, GDrugPaths::IDENTIFIED_MPID_VERSION);
		let version = input_string(
			first_attr(xpath, node, GDrugPaths::FDA_MPID_VERSION),
			"mpidVersion",
			input_contracts::generated::g::fda_g_k_2_1_1a,
			xpath,
		)?;
		if identified_version.is_some() && identified_version != version {
			return Err(Error::InvalidImportRequest {
				message: "G.k.2.1.1a contains conflicting FDA and MPID versions"
					.to_owned(),
			});
		}
		xpath.inputs.string(
			"mpid",
			Some(&mpid),
			None,
			input_contracts::generated::g::g_k_2_1_1b,
		);
		return Ok((
			Some(mpid),
			version.clone(),
			Some("2.16.840.1.113883.6.69".to_string()),
			version,
		));
	}
	Ok((
		read_g_k_2_1_1a(xpath, node)?,
		read_g_k_2_1_1b(xpath, node)?,
		None,
		None,
	))
}

/// e2b:G.k.2.1.1b
fn read_g_k_2_1_1b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::MPID_VERSION);
	xpath.inputs.string(
		"mpidVersion",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_2_1_1a,
	);
	Ok(value)
}

/// e2b:G.k.2.1.KR.1a
fn read_g_k_2_1_kr_1a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GMfdsDrugPaths::MPID_VERSION),
		"mfdsMpidVersion",
		input_contracts::generated::g::mfds_g_k_2_1_kr_1a,
		xpath,
	)
}

/// e2b:G.k.2.1.KR.1b
fn read_g_k_2_1_kr_1b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GMfdsDrugPaths::MPID),
		"mfdsMpid",
		input_contracts::generated::g::mfds_g_k_2_1_kr_1b,
		xpath,
	)
}

/// e2b:G.k.2.1.2a
fn read_g_k_2_1_2a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::PHPID);
	xpath.inputs.string(
		"phpid",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_2_1_2b,
	);
	Ok(value)
}

/// e2b:G.k.2.1.2b
fn read_g_k_2_1_2b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::PHPID_VERSION);
	xpath.inputs.string(
		"phpidVersion",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_2_1_2a,
	);
	Ok(value)
}

/// e2b:G.k.2.5
fn read_g_k_2_5(xpath: &mut Context, node: &Node) -> Result<Option<bool>> {
	let raw = first_attr(xpath, node, GDrugPaths::INVESTIGATIONAL_BLINDED);
	let value = parse_bool_literal(raw.as_deref());
	xpath.inputs.boolean(
		"investigationalProductBlinded",
		raw.as_deref(),
		value,
		None,
		input_contracts::generated::g::g_k_2_5,
	);
	Ok(value)
}

/// e2b:G.k.3.1
fn read_g_k_3_1(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::DRUG_AUTHORIZATION_NUMBER);
	xpath.inputs.string(
		"drugAuthorizationNumber",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_3_1,
	);
	Ok(value)
}

/// e2b:G.k.3.2
fn read_g_k_3_2(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_text(xpath, node, GDrugPaths::MANUFACTURER_NAME);
	xpath.inputs.string(
		"drugAuthorizationHolder",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_3_3,
	);
	Ok(value)
}

/// e2b:G.k.3.3
fn read_g_k_3_3(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::MANUFACTURER_COUNTRY);
	xpath.inputs.string(
		"drugAuthorizationCountry",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_3_2,
	);
	Ok(value)
}

/// e2b:G.k.2.4
fn read_g_k_2_4(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_text(xpath, node, GDrugPaths::OBTAIN_DRUG_COUNTRY);
	xpath.inputs.string(
		"obtainDrugCountry",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_2_4,
	);
	Ok(value)
}

/// e2b:G.k.8
fn read_g_k_8(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::ACTION_TAKEN);
	xpath.inputs.string(
		"drugActionTaken",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_8,
	);
	Ok(value)
}

/// e2b:G.k.local.batchLotNumber
fn read_g_k_local_batch_lot(xpath: &mut Context, node: &Node) -> Option<String> {
	first_text(xpath, node, GDrugPaths::BATCH_LOT_NUMBER)
}

/// e2b:G.k.5a
fn read_g_k_5a(xpath: &mut Context, node: &Node) -> Result<Option<Decimal>> {
	let raw = first_attr(xpath, node, GDrugPaths::CUMULATIVE_DOSE_VALUE);
	xpath.inputs.number_string(
		"cumulativeDoseValue",
		raw.as_deref(),
		input_contracts::generated::g::g_k_5a,
	);
	crate::import_constraint::decimal(raw, "numeric field")
}

/// e2b:G.k.5b
fn read_g_k_5b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::CUMULATIVE_DOSE_UNIT);
	xpath.inputs.string(
		"cumulativeDoseUnit",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_5b,
	);
	Ok(value)
}

/// e2b:G.k.6a
fn read_g_k_6a(xpath: &mut Context, node: &Node) -> Result<Option<Decimal>> {
	let raw = first_attr(xpath, node, GDrugPaths::GESTATION_EXPOSURE_VALUE);
	xpath.inputs.number_string(
		"gestationPeriodExposureValue",
		raw.as_deref(),
		input_contracts::generated::g::g_k_6a,
	);
	crate::import_constraint::decimal(raw, "numeric field")
}

/// e2b:G.k.6b
fn read_g_k_6b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_attr(xpath, node, GDrugPaths::GESTATION_EXPOSURE_UNIT);
	xpath.inputs.string(
		"gestationPeriodExposureUnit",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_6b,
	);
	Ok(value)
}

/// e2b:FDA.G.k.10.1
fn read_fda_g_k_10_1(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	let value = first_attr(xpath, node, GDrugPaths::FDA_ADDITIONAL_INFO);
	let null_flavor =
		first_attr(xpath, node, GDrugPaths::FDA_ADDITIONAL_INFO_NULL_FLAVOR);
	xpath.inputs.string(
		"fdaAdditionalInfoCoded",
		value.as_deref(),
		null_flavor.as_deref(),
		input_contracts::generated::g::fda_g_k_10a,
	);
	Ok((value, null_flavor))
}

/// e2b:FDA.G.k.10a
fn read_fda_g_k_10a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value =
		first_attr(xpath, node, GDrugPaths::FDA_SPECIALIZED_PRODUCT_CATEGORY);
	xpath.inputs.string(
		"fdaSpecializedProductCategory",
		value.as_deref(),
		None,
		input_contracts::generated::g::fda_g_k_10_1,
	);
	Ok(value)
}

/// e2b:G.k.10.r
fn read_g_k_10_r(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<serde_json::Value>> {
	let values =
		find_nodes(xpath, GDrugPaths::ICH_ADDITIONAL_INFO_VALUES, Some(node))?;
	let empty_collections =
		find_nodes(xpath, GDrugPaths::ICH_EMPTY_ADDITIONAL_INFO, Some(node))?;
	if !empty_collections.is_empty() {
		if empty_collections.len() != 1 || !values.is_empty() {
			return Err(invalid_field(
				"G.k.10.r",
				"Conflicting empty and repeated collection representations",
			));
		}
		return Ok(Some(serde_json::json!([])));
	}
	if values.is_empty() {
		return Ok(None);
	}
	let mut codes = Vec::with_capacity(values.len());
	for value in values {
		let code = value.get_property("code").ok_or_else(|| {
			invalid_field("G.k.10.r", "Coded value is missing code")
		})?;
		if code.trim().is_empty() {
			return Err(invalid_field("G.k.10.r", "Coded value has an empty code"));
		}
		let null_flavor = value.get_property("nullFlavor");
		xpath.inputs.string(
			"drugAdditionalInformationCodes[]",
			Some(&code),
			null_flavor.as_deref(),
			input_contracts::generated::g::g_k_10_r,
		);
		codes.push(serde_json::Value::String(code));
	}
	Ok(Some(serde_json::Value::Array(codes)))
}

/// e2b:G.k.11
fn read_g_k_11(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	let value = first_text(xpath, node, GDrugPaths::DRUG_ADDITIONAL_INFORMATION);
	xpath.inputs.string(
		"drugAdditionalInformation",
		value.as_deref(),
		None,
		input_contracts::generated::g::g_k_11,
	);
	Ok(value)
}

/// e2b:G.k.2.3.r.1
fn read_g_k_2_3_r_1(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, GDrugPaths::SUBSTANCE_NAME),
		"activeSubstances[].substanceName",
		input_contracts::generated::g::g_k_2_3_r_1,
		xpath,
	)
}

/// e2b:G.k.2.3.r.2a
fn read_g_k_2_3_r_2a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::SUBSTANCE_TERMID_VERSION),
		"activeSubstances[].substanceTermIdVersion",
		input_contracts::generated::g::g_k_2_3_r_2a,
		xpath,
	)
}

/// e2b:G.k.2.3.r.2b
fn read_g_k_2_3_r_2b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::SUBSTANCE_TERMID),
		"activeSubstances[].substanceTermId",
		input_contracts::generated::g::g_k_2_3_r_2b,
		xpath,
	)
}

/// XML code system metadata for G.k.2.3.r.2.
fn read_g_k_2_3_r_2_code_system(xpath: &mut Context, node: &Node) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::SUBSTANCE_TERMID_CODE_SYSTEM)
}

/// e2b:G.k.2.3.r.1.KR.1a
fn read_g_k_2_3_r_1_kr_1a(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GMfdsDrugPaths::SUBSTANCE_VERSION),
		"activeSubstances[].mfdsVersion",
		input_contracts::generated::g::mfds_g_k_2_3_r_1_kr_1a,
		xpath,
	)
}

/// e2b:G.k.2.3.r.1.KR.1b
fn read_g_k_2_3_r_1_kr_1b(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GMfdsDrugPaths::SUBSTANCE_ID),
		"activeSubstances[].mfdsId",
		input_contracts::generated::g::mfds_g_k_2_3_r_1_kr_1b,
		xpath,
	)
}

/// e2b:G.k.2.3.r.3a
fn read_g_k_2_3_r_3a(xpath: &mut Context, node: &Node) -> Result<Option<Decimal>> {
	decimal(
		first_attr(xpath, node, GDrugPaths::SUBSTANCE_STRENGTH_VALUE),
		"activeSubstances[].substanceStrengthValue",
		input_contracts::generated::g::g_k_2_3_r_3a,
		xpath,
	)
}

/// e2b:G.k.2.3.r.3b
fn read_g_k_2_3_r_3b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::SUBSTANCE_STRENGTH_UNIT),
		"activeSubstances[].substanceStrengthUnit",
		input_contracts::generated::g::g_k_2_3_r_3b,
		xpath,
	)
}

/// e2b:G.k.4.r.1a
fn read_g_k_4_r_1a(xpath: &mut Context, node: &Node) -> Result<Option<Decimal>> {
	decimal(
		first_attr(xpath, node, GDrugPaths::DOSE_VALUE),
		"dosageInformation[].doseValue",
		input_contracts::generated::g::g_k_4_r_1a,
		xpath,
	)
}

/// e2b:G.k.4.r.1b
fn read_g_k_4_r_1b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DOSE_UNIT),
		"dosageInformation[].doseUnit",
		input_contracts::generated::g::g_k_4_r_1b,
		xpath,
	)
}

/// e2b:G.k.4.r.2
fn read_g_k_4_r_2(xpath: &mut Context, node: &Node) -> Result<Option<Decimal>> {
	decimal(
		first_attr(xpath, node, GDrugPaths::DOSAGE_FREQUENCY_VALUE),
		"dosageInformation[].numberOfUnits",
		input_contracts::generated::g::g_k_4_r_2,
		xpath,
	)
}

/// e2b:G.k.4.r.3
fn read_g_k_4_r_3(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DOSAGE_FREQUENCY_UNIT),
		"dosageInformation[].frequencyUnit",
		input_contracts::generated::g::g_k_4_r_3,
		xpath,
	)
}

/// e2b:G.k.4.r.4
fn read_g_k_4_r_4(
	xpath: &mut Context,
	node: &Node,
) -> Result<(
	Option<sqlx::types::time::Date>,
	Option<String>,
	Option<String>,
)> {
	date_pair(
		first_attr(xpath, node, GDrugPaths::DOSAGE_START_DATE),
		first_attr(xpath, node, GDrugPaths::DOSAGE_START_DATE_NULL_FLAVOR).or_else(
			|| {
				first_attr(
					xpath,
					node,
					GDrugPaths::DOSAGE_EFFECTIVE_TIME_NULL_FLAVOR,
				)
			},
		),
		"dosageInformation[].firstAdministrationDate",
		"dosageInformation[].firstAdministrationDateNullFlavor",
		input_contracts::generated::g::g_k_4_r_4,
		xpath,
	)
}

/// e2b:G.k.4.r.5
fn read_g_k_4_r_5(
	xpath: &mut Context,
	node: &Node,
) -> Result<(
	Option<sqlx::types::time::Date>,
	Option<String>,
	Option<String>,
)> {
	date_pair(
		first_attr(xpath, node, GDrugPaths::DOSAGE_END_DATE),
		first_attr(xpath, node, GDrugPaths::DOSAGE_END_DATE_NULL_FLAVOR).or_else(
			|| {
				first_attr(
					xpath,
					node,
					GDrugPaths::DOSAGE_EFFECTIVE_TIME_NULL_FLAVOR,
				)
			},
		),
		"dosageInformation[].lastAdministrationDate",
		"dosageInformation[].lastAdministrationDateNullFlavor",
		input_contracts::generated::g::g_k_4_r_5,
		xpath,
	)
}

/// e2b:G.k.4.r.6a
fn read_g_k_4_r_6a(xpath: &mut Context, node: &Node) -> Result<Option<Decimal>> {
	decimal(
		first_attr(xpath, node, GDrugPaths::DOSAGE_DURATION_VALUE),
		"dosageInformation[].durationValue",
		input_contracts::generated::g::g_k_4_r_6a,
		xpath,
	)
}

/// e2b:G.k.4.r.6b
fn read_g_k_4_r_6b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DOSAGE_DURATION_UNIT),
		"dosageInformation[].durationUnit",
		input_contracts::generated::g::g_k_4_r_6b,
		xpath,
	)
}

/// e2b:G.k.4.r.7
fn read_g_k_4_r_7(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	let pair = input_string_pair(
		first_text(xpath, node, GDrugPaths::DOSAGE_BATCH_LOT),
		first_attr(xpath, node, GDrugPaths::DOSAGE_BATCH_LOT_NULL_FLAVOR),
		"dosageInformation[].batchNumber",
		"dosageInformation[].batchNumberNullFlavor",
		input_contracts::generated::g::g_k_4_r_7,
		xpath,
	)?;

	Ok(pair)
}

/// e2b:G.k.4.r.8
fn read_g_k_4_r_8(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, GDrugPaths::DOSAGE_TEXT_NODE),
		"dosageInformation[].dosageText",
		input_contracts::generated::g::g_k_4_r_8,
		xpath,
	)
}

/// e2b:G.k.4.r.9.1
fn read_g_k_4_r_9_1(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	input_string_pair(
		first_text(xpath, node, GDrugPaths::DOSE_FORM_TEXT),
		first_attr(xpath, node, GDrugPaths::DOSE_FORM_NULL_FLAVOR),
		"dosageInformation[].doseForm",
		"dosageInformation[].doseFormNullFlavor",
		input_contracts::generated::g::g_k_4_r_9_1,
		xpath,
	)
}

/// e2b:G.k.4.r.9.2a
fn read_g_k_4_r_9_2a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DOSE_FORM_TERMID_VERSION),
		"dosageInformation[].doseFormTermIdVersion",
		input_contracts::generated::g::g_k_4_r_9_2a,
		xpath,
	)
}

/// e2b:G.k.4.r.9.2b
fn read_g_k_4_r_9_2b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DOSE_FORM_TERMID),
		"dosageInformation[].doseFormTermId",
		input_contracts::generated::g::g_k_4_r_9_2b,
		xpath,
	)
}

/// e2b:G.k.4.r.10.1
fn read_g_k_4_r_10_1(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	input_string_pair(
		first_text(xpath, node, GDrugPaths::ROUTE_TEXT),
		first_attr(xpath, node, GDrugPaths::ROUTE_NULL_FLAVOR),
		"dosageInformation[].routeOfAdministration",
		"dosageInformation[].routeOfAdministrationNullFlavor",
		input_contracts::generated::g::g_k_4_r_10_1,
		xpath,
	)
}

/// e2b:G.k.4.r.10.2a
fn read_g_k_4_r_10_2a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::ROUTE_CODE_SYSTEM_VERSION),
		"dosageInformation[].routeTermIdVersion",
		input_contracts::generated::g::g_k_4_r_10_2a,
		xpath,
	)
}

/// e2b:G.k.4.r.10.2b
fn read_g_k_4_r_10_2b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::ROUTE_CODE),
		"dosageInformation[].routeTermId",
		input_contracts::generated::g::g_k_4_r_10_2b,
		xpath,
	)
}

fn read_g_k_4_r_10_2_code_system(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	Ok(first_attr(xpath, node, GDrugPaths::ROUTE_CODE_SYSTEM))
}

/// e2b:G.k.4.r.11.1
fn read_g_k_4_r_11_1(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	input_string_pair(
		first_text(xpath, node, GDrugPaths::DOSAGE_PARENT_ROUTE_TEXT),
		first_attr(xpath, node, GDrugPaths::DOSAGE_PARENT_ROUTE_NULL_FLAVOR),
		"dosageInformation[].parentRouteOfAdministration",
		"dosageInformation[].parentRouteOfAdministrationNullFlavor",
		input_contracts::generated::g::g_k_4_r_11_1,
		xpath,
	)
}

/// e2b:G.k.4.r.11.2a
fn read_g_k_4_r_11_2a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DOSAGE_PARENT_ROUTE_TERMID_VERSION),
		"dosageInformation[].parentRouteTermIdVersion",
		input_contracts::generated::g::g_k_4_r_11_2a,
		xpath,
	)
}

/// e2b:G.k.4.r.11.2b
fn read_g_k_4_r_11_2b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DOSAGE_PARENT_ROUTE_TERMID),
		"dosageInformation[].parentRouteTermId",
		input_contracts::generated::g::g_k_4_r_11_2b,
		xpath,
	)
}

fn read_g_k_4_r_11_2_code_system(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	Ok(first_attr(
		xpath,
		node,
		GDrugPaths::DOSAGE_PARENT_ROUTE_TERMID_CODE_SYSTEM,
	))
}

/// e2b:G.k.7.r.1
fn read_g_k_7_r_1(
	xpath: &mut Context,
	node: &Node,
) -> Result<(Option<String>, Option<String>)> {
	input_string_pair(
		first_text(xpath, node, GDrugPaths::INDICATION_TEXT),
		first_attr(xpath, node, GDrugPaths::INDICATION_TEXT_NULL_FLAVOR),
		"indications[].indicationText",
		"indications[].indicationTextNullFlavor",
		input_contracts::generated::g::g_k_7_r_1,
		xpath,
	)
}

/// e2b:G.k.7.r.2a
fn read_g_k_7_r_2a(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::INDICATION_VERSION),
		"indications[].indicationMeddraVersion",
		input_contracts::generated::g::g_k_7_r_2a,
		xpath,
	)
}

/// e2b:G.k.7.r.2b
fn read_g_k_7_r_2b(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::INDICATION_CODE),
		"indications[].indicationMeddraCode",
		input_contracts::generated::g::g_k_7_r_2b,
		xpath,
	)
}

/// e2b:FDA.G.k.local.deviceCharacteristic.code
fn read_device_characteristic_code(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::DEVICE_CHAR_CODE)
}

/// e2b:FDA.G.k.local.deviceCharacteristic.codeSystem
fn read_device_characteristic_code_system(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::DEVICE_CHAR_CODE_SYSTEM)
}

/// e2b:FDA.G.k.local.deviceCharacteristic.codeDisplayName
fn read_device_characteristic_code_display_name(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::DEVICE_CHAR_DISPLAY)
}

/// e2b:FDA.G.k.local.deviceCharacteristic.valueType
fn read_device_characteristic_value_type(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::DEVICE_CHAR_VALUE_TYPE)
}

/// e2b:FDA.G.k.local.deviceCharacteristic.valueValue
fn read_device_characteristic_value_value(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::DEVICE_CHAR_VALUE_VALUE)
}

/// e2b:FDA.G.k.local.deviceCharacteristic.valueCode
fn read_device_characteristic_value_code(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::DEVICE_CHAR_VALUE_CODE)
}

/// e2b:FDA.G.k.local.deviceCharacteristic.valueCodeSystem
fn read_device_characteristic_value_code_system(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::DEVICE_CHAR_VALUE_CODE_SYSTEM)
}

/// e2b:FDA.G.k.local.deviceCharacteristic.valueDisplayName
fn read_device_characteristic_value_display_name(
	xpath: &mut Context,
	node: &Node,
) -> Option<String> {
	first_attr(xpath, node, GDrugPaths::DEVICE_CHAR_VALUE_DISPLAY)
}

/// e2b:FDA.G.k.12.r.1
fn read_fda_g_k_12_r_1(xpath: &mut Context, node: &Node) -> Result<Option<bool>> {
	let raw = first_attr(xpath, node, GDrugPaths::DEVICE_MALFUNCTION);
	let value = parse_bool_literal(raw.as_deref());
	Ok(value)
}

fn read_fda_device_code(
	xpath: &mut Context,
	node: &Node,
	code: &str,
) -> Option<String> {
	(first_attr(xpath, node, "hl7:code/@code").as_deref() == Some(code))
		.then(|| first_attr(xpath, node, "hl7:value/@code"))
		.flatten()
}

/// e2b:FDA.G.k.12.r.2.r
fn read_fda_g_k_12_r_2_r(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	if first_attr(xpath, node, "hl7:code/@code").as_deref() != Some("C54592") {
		return Ok(None);
	}
	input_string(
		read_fda_device_code(xpath, node, "C54592"),
		"fdaDevices[].followUpTypes[].valueCode",
		input_contracts::generated::g::fda_g_k_12_r_2_r,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.3.r
fn read_fda_g_k_12_r_3_r(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	if first_attr(xpath, node, "hl7:code/@code").as_deref() != Some("C54451") {
		return Ok(None);
	}
	input_string(
		read_fda_device_code(xpath, node, "C54451"),
		"fdaDevices[].deviceProblemCodes[].valueCode",
		input_contracts::generated::g::fda_g_k_12_r_3_r,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.4
fn read_fda_g_k_12_r_4(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, "hl7:partProduct/hl7:name[1]"),
		"fdaDevices[].deviceBrandName",
		input_contracts::generated::g::fda_g_k_12_r_4,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.5
fn read_fda_g_k_12_r_5(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, "hl7:partProduct/hl7:name[2]"),
		"fdaDevices[].commonDeviceName",
		input_contracts::generated::g::fda_g_k_12_r_5,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.6
fn read_fda_g_k_12_r_6(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, "hl7:partProduct/hl7:code/@code"),
		"fdaDevices[].deviceProductCode",
		input_contracts::generated::g::fda_g_k_12_r_6,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.7.1a
fn read_fda_g_k_12_r_7_1a(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, GDrugPaths::DEVICE_MANUFACTURER_NAME),
		"fdaDevices[].manufacturerName",
		input_contracts::generated::g::fda_g_k_12_r_7_1a,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.7.1b
fn read_fda_g_k_12_r_7_1b(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, GDrugPaths::DEVICE_MANUFACTURER_ADDRESS),
		"fdaDevices[].manufacturerAddress",
		input_contracts::generated::g::fda_g_k_12_r_7_1b,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.7.1c
fn read_fda_g_k_12_r_7_1c(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, GDrugPaths::DEVICE_MANUFACTURER_CITY),
		"fdaDevices[].manufacturerCity",
		input_contracts::generated::g::fda_g_k_12_r_7_1c,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.7.1d
fn read_fda_g_k_12_r_7_1d(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, GDrugPaths::DEVICE_MANUFACTURER_STATE),
		"fdaDevices[].manufacturerState",
		input_contracts::generated::g::fda_g_k_12_r_7_1d,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.7.1e
fn read_fda_g_k_12_r_7_1e(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, GDrugPaths::DEVICE_MANUFACTURER_COUNTRY),
		"fdaDevices[].manufacturerCountry",
		input_contracts::generated::g::fda_g_k_12_r_7_1e,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.8
fn read_fda_g_k_12_r_8(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DEVICE_USAGE),
		"fdaDevices[].deviceUsage",
		input_contracts::generated::g::fda_g_k_12_r_8,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.9
fn read_fda_g_k_12_r_9(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_text(xpath, node, GDrugPaths::DEVICE_LOT_NUMBER),
		"fdaDevices[].deviceLotNumber",
		input_contracts::generated::g::fda_g_k_12_r_9,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.10
fn read_fda_g_k_12_r_10(xpath: &mut Context, node: &Node) -> Result<Option<String>> {
	input_string(
		first_attr(xpath, node, GDrugPaths::DEVICE_OPERATOR_CODE),
		"fdaDevices[].operatorOfDevice",
		input_contracts::generated::g::fda_g_k_12_r_10,
		xpath,
	)
}

/// e2b:FDA.G.k.12.r.11.r
fn read_fda_g_k_12_r_11_r(
	xpath: &mut Context,
	node: &Node,
) -> Result<Option<String>> {
	if first_attr(xpath, node, "hl7:code/@code").as_deref() != Some("C54594") {
		return Ok(None);
	}
	input_string(
		read_fda_device_code(xpath, node, "C54594"),
		"fdaDevices[].remedialActions[].valueCode",
		input_contracts::generated::g::fda_g_k_12_r_11_r,
		xpath,
	)
}

fn first_attr(xpath: &mut Context, node: &Node, expr: &str) -> Option<String> {
	xpath.findvalues(expr, Some(node)).ok()?.into_iter().next()
}

fn find_nodes(
	xpath: &mut Context,
	expr: &str,
	node: Option<&Node>,
) -> Result<Vec<Node>> {
	xpath.findnodes(expr, node).map_err(|_| Error::InvalidXml {
		message: format!("Failed to query XML path: {expr}"),
		line: None,
		column: None,
	})
}

fn invalid_field(field: &str, detail: impl std::fmt::Display) -> Error {
	Error::InvalidXml {
		message: format!("ICH.{field}: {detail}"),
		line: None,
		column: None,
	}
}

fn decimal(
	value: Option<String>,
	field: &str,
	check: input_contracts::Check,
	xpath: &mut Context,
) -> Result<Option<Decimal>> {
	xpath.inputs.number_string(field, value.as_deref(), check);
	crate::import_constraint::decimal(value, field)
}

fn input_string(
	value: Option<String>,
	field: &str,
	check: input_contracts::Check,
	xpath: &mut Context,
) -> Result<Option<String>> {
	xpath.inputs.string(field, value.as_deref(), None, check);
	Ok(value)
}

fn input_string_pair(
	value: Option<String>,
	null_flavor: Option<String>,
	field: &str,
	_null_flavor_field: &str,
	check: input_contracts::Check,
	xpath: &mut Context,
) -> Result<(Option<String>, Option<String>)> {
	xpath
		.inputs
		.string(field, value.as_deref(), null_flavor.as_deref(), check);
	Ok((value, null_flavor))
}

fn date_pair(
	value: Option<String>,
	null_flavor: Option<String>,
	field: &str,
	null_flavor_field: &str,
	check: input_contracts::Check,
	xpath: &mut Context,
) -> Result<(
	Option<sqlx::types::time::Date>,
	Option<String>,
	Option<String>,
)> {
	let (value, null_flavor) = input_string_pair(
		value,
		null_flavor,
		field,
		null_flavor_field,
		check,
		xpath,
	)?;
	let Some(value) = value else {
		return Ok((None, None, null_flavor));
	};
	let (date, raw) = parse_date(value)
		.ok_or_else(|| invalid_field(field, "date cannot be stored without loss"))?;
	Ok((date, raw, null_flavor))
}

fn parse_date(
	value: String,
) -> Option<(Option<sqlx::types::time::Date>, Option<String>)> {
	if value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_digit()) {
		crate::import_sections::shared::parse_date(value)
			.map(|date| (Some(date), None))
	} else {
		// Keep timestamp precision and blank draft values; the record validates the lexeme.
		Some((None, Some(value)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const TEST_DRUG_ID: &str = "3c91b4d5-e039-4a7a-9c30-67671b0ef9e4";

	#[test]
	fn parses_partial_hl7_ts_dates_without_losing_precision() {
		let (date, raw) = parse_date("200304".to_string()).expect("partial date");
		assert_eq!(date, None);
		assert_eq!(raw.as_deref(), Some("200304"));
	}

	#[test]
	fn raw_date_conversion_does_not_discard_the_source_lexeme() {
		assert_eq!(
			parse_date("2003-04".to_string()),
			Some((None, Some("2003-04".into())))
		);
	}

	fn with_drug_role(xml: &str) -> String {
		let xml = xml.replacen(
			"<substanceAdministration>",
			&format!("<substanceAdministration><id root=\"{TEST_DRUG_ID}\"/>"),
			1,
		);
		xml.replacen(
			"</MCCI_IN200100UV01>",
			&format!(
				"<component><causalityAssessment><code code=\"20\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.13\"/><subject2><productUseReference><id root=\"{TEST_DRUG_ID}\"/></productUseReference></subject2></causalityAssessment></component></MCCI_IN200100UV01>"
			),
			1,
		)
	}

	#[test]
	fn preserves_ich_additional_codes_separately_from_fda() {
		let values = ["2", "10", "2"].map(|code| format!(r#"<outboundRelationship2 typeCode="REFR"><observation><code code="9" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value codeSystem="2.16.840.1.113883.3.989.2.1.1.17" code="{code}"/></observation></outboundRelationship2>"#)).join("");
		for fda in [
			"",
			r#"<outboundRelationship2 typeCode="REFR"><observation><code code="9"/><value code="1"/></observation></outboundRelationship2>"#,
		] {
			let xml = with_drug_role(&format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable>{values}{fda}</substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#
			));
			let drugs = parse_g_drugs(xml.as_bytes()).expect("isolated XML fixture");
			assert_eq!(drugs.len(), 1);
			assert_eq!(
				drugs[0].drug_additional_info_codes_json,
				Some(serde_json::json!(["2", "10", "2"]))
			);
			assert_eq!(
				drugs[0].fda_additional_info_coded.as_deref(),
				if fda.is_empty() { None } else { Some("1") }
			);
		}
	}

	#[test]
	fn local_product_links_reject_ambiguous_or_misplaced_references() {
		use product_links::SourceProductLink;
		let xml = |markers: &str| {
			with_drug_role(&format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration>{markers}<consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#
			))
		};
		let marker = format!("<?e2br3-product-link-v1 {TEST_DRUG_ID}?>");
		for (metadata, expected) in [
			(String::new(), SourceProductLink::Legacy),
			(
				"<?e2br3-product-link-v1 none?>".to_owned(),
				SourceProductLink::Unlinked,
			),
			(
				marker.clone(),
				SourceProductLink::Linked(
					uuid::Uuid::parse_str(TEST_DRUG_ID).expect("test UUID"),
				),
			),
		] {
			assert_eq!(
				parse_g_drugs(xml(&metadata).as_bytes()).expect("parse metadata")[0]
					.source_product_link,
				expected
			);
		}
		for metadata in ["<?e2br3-product-link-v1 ?>".to_owned(), "<?e2br3-product-link-v1 00000000-0000-0000-0000-000000000000?>".to_owned(), "<?e2br3-product-link-v1 invalid?>".to_owned(), "<?e2br3-product-link-v2 none?>".to_owned(), format!("{marker}{marker}"), format!("<outboundRelationship2><substanceAdministration>{marker}</substanceAdministration></outboundRelationship2>")] {
			assert!(parse_g_drugs(xml(&metadata).as_bytes()).is_err(), "Malformed or misplaced references must not be discarded");
		}
		let orphan = xml("").replace(
			"</MCCI_IN200100UV01>",
			&format!("{marker}</MCCI_IN200100UV01>"),
		);
		assert!(parse_g_drugs(orphan.as_bytes()).is_err());
	}

	#[test]
	fn local_source_product_metadata_rejects_invalid_or_misplaced_values() {
		use base64::Engine as _;
		let xml = |markers: &str| {
			with_drug_role(&format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration>{markers}<consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#
			))
		};
		let encode =
			|bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
		for invalid in [
			"%%%".to_owned(),
			"YQ".to_owned(),
			"/w==".to_owned(),
			encode(b"a\0b"),
			encode("한".repeat(256).as_bytes()),
		] {
			assert!(
				parse_g_drugs(
					xml(&format!("<?e2br3-source-product-id-v1 {invalid}?>"))
						.as_bytes()
				)
				.is_err(),
				"Invalid local string must not be discarded"
			);
		}
		for markers in ["<?e2br3-source-product-id-v2 YQ==?>", "<?e2br3-source-product-id-v1 YQ==?><?e2br3-source-product-id-v1 YQ==?>", "<outboundRelationship2 typeCode=\"COMP\"><substanceAdministration><?e2br3-source-product-id-v1 YQ==?></substanceAdministration></outboundRelationship2>"] {
			assert!(parse_g_drugs(xml(markers).as_bytes()).is_err(), "Unsupported, duplicate or misplaced local string");
		}
		let misplaced = xml("").replace(
			"</MCCI_IN200100UV01>",
			"<?e2br3-source-product-id-v1 YQ==?></MCCI_IN200100UV01>",
		);
		assert!(parse_g_drugs(misplaced.as_bytes()).is_err());
	}

	#[test]
	fn local_continuing_metadata_rejects_ambiguous_invalid_or_misplaced_values() {
		let xml = |markers: &str| {
			with_drug_role(&format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable><outboundRelationship2 typeCode="COMP"><substanceAdministration>{markers}</substanceAdministration></outboundRelationship2></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#
			))
		};
		for invalid in ["", "0", "1", "TRUE", "null", "yes", "false extra"] {
			assert!(
				parse_g_drugs(
					xml(&format!("<?e2br3-dosage-continuing-v1 {invalid}?>"))
						.as_bytes()
				)
				.is_err(),
				"Invalid local value: {invalid}"
			);
		}
		for markers in ["<?e2br3-dosage-continuing-v2 false?>", "<?e2br3-dosage-continuing-v1 false?><?e2br3-dosage-continuing-v1 false?>", "<?e2br3-dosage-continuing-v1 true?><?e2br3-dosage-continuing-v1 false?>"] {
			assert!(parse_g_drugs(xml(markers).as_bytes()).is_err(), "Unsupported/ambiguous local metadata");
		}
		let misplaced = xml("").replace(
			"</MCCI_IN200100UV01>",
			"<?e2br3-dosage-continuing-v1 false?></MCCI_IN200100UV01>",
		);
		assert!(
			parse_g_drugs(misplaced.as_bytes()).is_err(),
			"Never silently drop misplaced local data"
		);
	}

	#[test]
	fn additional_codes_preserve_explicit_empty_collection_without_inference() {
		let observation = r#"<outboundRelationship2 typeCode="REFR"><observation classCode="OBS" moodCode="EVN"><code code="9" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/></observation></outboundRelationship2>"#;
		let xml = |value: &str| {
			with_drug_role(&format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable>{value}</substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#
			))
		};
		let empty = parse_g_drugs(xml(observation).as_bytes())
			.expect("explicit empty collection");
		assert_eq!(
			empty[0].drug_additional_info_codes_json,
			Some(serde_json::json!([]))
		);
		for absent in [
			String::new(),
			observation
				.replace(" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"", ""),
		] {
			let imported = parse_g_drugs(xml(&absent).as_bytes())
				.expect("not an ICH empty collection");
			assert_eq!(imported[0].drug_additional_info_codes_json, None);
		}
		let populated = observation.replace("</observation>", "<value codeSystem=\"2.16.840.1.113883.3.989.2.1.1.17\" code=\"2\"/></observation>");
		for conflicting in [
			format!("{observation}{observation}"),
			format!("{observation}{populated}"),
		] {
			assert!(
				parse_g_drugs(xml(&conflicting).as_bytes()).is_err(),
				"Conflicting collection representations must not be silently merged"
			);
		}
	}

	#[test]
	fn additional_codes_keep_absence_and_reject_malformed_values() {
		let xml = |value: &str| {
			with_drug_role(&format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable>{value}</substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#
			))
		};
		let absent =
			parse_g_drugs(xml("").as_bytes()).expect("absent optional codes");
		assert_eq!(absent[0].drug_additional_info_codes_json, None);
		for attributes in [
			"",
			r#"code="""#,
			r#"code="0""#,
			r#"code="123""#,
			r#"code="1" nullFlavor="NI""#,
		] {
			let value = format!(
				r#"<outboundRelationship2 typeCode="REFR"><observation><code code="9"/><value codeSystem="2.16.840.1.113883.3.989.2.1.1.17" {attributes}/></observation></outboundRelationship2>"#
			);
			assert!(
				parse_g_drugs(xml(&value).as_bytes()).is_err(),
				"{attributes}"
			);
		}
	}

	#[test]
	fn rejects_unstorable_lengths_and_unparseable_numbers() {
		let overlong_version = format!(
			r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name><asIdentifiedEntity><id extension="MPID-1"/><code code="MPID" codeSystemVersion="{}"/></asIdentifiedEntity></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#,
			"1".repeat(1001)
		);
		let invalid_decimal = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable><outboundRelationship2 typeCode="COMP"><substanceAdministration><doseQuantity value="not-a-number"/></substanceAdministration></outboundRelationship2></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#;

		assert!(parse_g_drugs(with_drug_role(&overlong_version).as_bytes()).is_err());
		assert!(parse_g_drugs(
			with_drug_role(std::str::from_utf8(invalid_decimal).unwrap()).as_bytes()
		)
		.is_err());
	}

	#[test]
	fn rejects_drug_without_characterization_reference_before_db_write() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><id root="drug-without-role"/><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#;

		let error = parse_g_drugs(xml).expect_err("missing G.k.1");
		assert!(error.to_string().contains("G.k.1"));
	}

	#[test]
	fn imports_decimal_interval_value_and_special_frequency_unit() {
		for unit in ["{cyclical}", "{asnecessary}", "{total}"] {
			let xml = format!(
				r#"
			<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"
				xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
				<subjectOf2>
					<organizer>
						<code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/>
						<component>
							<substanceAdministration>
								<consumable><instanceOfKind><kindOfProduct>
									<name>Drug A</name>
								</kindOfProduct></instanceOfKind></consumable>
								<outboundRelationship2 typeCode="COMP">
									<substanceAdministration>
										<effectiveTime><comp xsi:type="PIVL_TS">
											<period value="0.5" unit="{unit}"/>
										</comp></effectiveTime>
									</substanceAdministration>
								</outboundRelationship2>
							</substanceAdministration>
						</component>
					</organizer>
				</subjectOf2>
			</MCCI_IN200100UV01>
		"#
			);

			let drugs =
				parse_g_drugs(with_drug_role(&xml).as_bytes()).expect("parse");
			let dosage = &drugs[0].dosages[0];

			assert_eq!(dosage.number_of_units, Some(Decimal::new(5, 1)));
			assert_eq!(dosage.frequency_unit.as_deref(), Some(unit));
		}
	}

	#[test]
	fn imports_dosage_null_flavors_into_companions() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable><outboundRelationship2 typeCode="COMP"><substanceAdministration><routeCode nullFlavor="ASKU"/><consumable><instanceOfKind><kindOfProduct><formCode nullFlavor="UNK"/></kindOfProduct></instanceOfKind></consumable><outboundRelationship2 typeCode="COMP"><observation><code code="G.k.4.r.11"/><value nullFlavor="NASK"/></observation></outboundRelationship2></substanceAdministration></outboundRelationship2></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#;
		let drugs = parse_g_drugs(
			with_drug_role(std::str::from_utf8(xml).unwrap()).as_bytes(),
		)
		.expect("parse");
		let dosage = &drugs[0].dosages[0];
		assert_eq!(dosage.route_null_flavor.as_deref(), Some("ASKU"));
		assert_eq!(dosage.dose_form_null_flavor.as_deref(), Some("UNK"));
		assert_eq!(dosage.parent_route_null_flavor.as_deref(), Some("NASK"));
	}

	#[test]
	fn imports_dosage_route_code_systems() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable><outboundRelationship2 typeCode="COMP"><substanceAdministration><routeCode code="001" codeSystem="0.4.0.127.0.16.1.1.2.6"/><outboundRelationship2 typeCode="COMP"><observation><code code="G.k.4.r.11"/><value code="002" codeSystem="2.16.840.1.113883.3.989.2.1.1.14"/></observation></outboundRelationship2></substanceAdministration></outboundRelationship2></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#;
		let drugs = parse_g_drugs(
			with_drug_role(std::str::from_utf8(xml).unwrap()).as_bytes(),
		)
		.expect("parse route code systems");
		let dosage = &drugs[0].dosages[0];
		assert_eq!(
			dosage.route_termid_code_system.as_deref(),
			Some("0.4.0.127.0.16.1.1.2.6")
		);
		assert_eq!(
			dosage.parent_route_termid_code_system.as_deref(),
			Some("2.16.840.1.113883.3.989.2.1.1.14")
		);
	}

	#[test]
	fn imports_batch_lot_null_flavor_and_rejects_a_value_pair() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable><outboundRelationship2 typeCode="COMP"><substanceAdministration><consumable><instanceOfKind><productInstanceInstance><lotNumberText nullFlavor="UNK"/></productInstanceInstance></instanceOfKind></consumable></substanceAdministration></outboundRelationship2></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#;

		let drugs = parse_g_drugs(
			with_drug_role(std::str::from_utf8(xml).unwrap()).as_bytes(),
		)
		.expect("parse batch lot NullFlavor");
		assert_eq!(drugs[0].dosages[0].batch_lot, None);
		assert_eq!(
			drugs[0].dosages[0].batch_lot_null_flavor.as_deref(),
			Some("UNK")
		);

		let invalid = std::str::from_utf8(xml).unwrap().replace(
			"<lotNumberText nullFlavor=\"UNK\"/>",
			"<lotNumberText nullFlavor=\"UNK\">LOT-1</lotNumberText>",
		);
		assert!(parse_g_drugs(with_drug_role(&invalid).as_bytes()).is_err());

		let unsupported = std::str::from_utf8(xml)
			.unwrap()
			.replace("nullFlavor=\"UNK\"", "nullFlavor=\"NI\"");
		assert!(parse_g_drugs(with_drug_role(&unsupported).as_bytes()).is_err());
	}

	#[test]
	fn imports_official_fda_scenario_7_device_repeat_groups() {
		let xml = include_str!(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/../../../docs/exporter/fda/FAERS2022Scenario7.xml"
		))
		.replace("201411011202", "2014110112");
		let drugs = parse_g_drugs(xml.as_bytes()).expect("parse FDA scenario 7");
		let devices: Vec<_> = drugs.iter().flat_map(|drug| &drug.devices).collect();

		assert_eq!(devices.len(), 2);
		assert!(devices
			.iter()
			.all(|device| device.malfunction == Some(true)));
		assert!(devices.iter().all(|device| device
			.codes
			.iter()
			.any(|code| code.element == "device_problem")));
		let drug_b = drugs
			.iter()
			.find(|drug| drug.medicinal_product == "Drug B")
			.expect("Drug B");
		assert_eq!(drug_b.fda_other_characterization.as_deref(), Some("1"));
	}

	#[test]
	fn fda_device_code_contract_applies_only_to_the_matching_characteristic() {
		for (code, element, value) in [
			("C54592", "follow_up_type", "1"),
			("C54451", "device_problem", "C1234"),
			("C54594", "remedial_action", "2"),
		] {
			let characteristic = format!(
				"<characteristic><code code=\"{code}\"/><value code=\"{value}\"/></characteristic>"
			);
			let xml = format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><name>Drug A</name><part><partProduct classCode="DEV"><asManufacturedProduct><subjectOf>{characteristic}</subjectOf></asManufacturedProduct></partProduct></part></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#
			);
			let drugs = parse_g_drugs(with_drug_role(&xml).as_bytes())
				.expect("unrelated absent device-code elements are not required");
			assert!(drugs[0].devices[0]
				.codes
				.iter()
				.any(|entry| entry.element == element && entry.value_code == value));

			let missing_value =
				xml.replace(&format!("<value code=\"{value}\"/>"), "<value/>");
			let error = parse_g_drugs(with_drug_role(&missing_value).as_bytes())
				.expect_err(
					"matching device-code element rejects an empty value code",
				);
			assert!(
				error.to_string().contains("INPUT.EMPTY.REJECTED"),
				"{error}"
			);
		}
	}

	#[test]
	fn fda_mpid_rejects_conflicts_and_versions_over_twenty_characters() {
		for (canonical_version, identifier, identified_version, expected) in [
			(
				"201411011202",
				"OTHER",
				"201411011202",
				"conflicting FDA and MPID identifiers",
			),
			(
				"201411011202",
				"59762-2858",
				"1",
				"conflicting FDA and MPID versions",
			),
			(
				"123456789012345678901",
				"59762-2858",
				"123456789012345678901",
				"FDA.G.k.2.1.1a.LENGTH.MAX",
			),
		] {
			let xml = with_drug_role(&format!(
				r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><code code="59762-2858" codeSystem="2.16.840.1.113883.6.69" codeSystemVersion="{canonical_version}"/><name>Drug A</name><asIdentifiedEntity><id extension="{identifier}"/><code code="MPID" codeSystemVersion="{identified_version}"/></asIdentifiedEntity></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#
			));
			let error = parse_g_drugs(xml.as_bytes())
				.expect_err("reject ambiguous or invalid MPID");
			assert!(error.to_string().contains(expected), "{error}");
		}
	}

	#[test]
	fn fda_canonical_mpid_version_populates_editor_and_source_metadata() {
		let xml = with_drug_role(
			r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><consumable><instanceOfKind><kindOfProduct><code code="59762-2858" codeSystem="2.16.840.1.113883.6.69" codeSystemVersion="201411011202"/><name>Drug A</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2></MCCI_IN200100UV01>"#,
		);
		let drug = &parse_g_drugs(xml.as_bytes()).expect("parse FDA MPID")[0];
		assert_eq!(drug.mpid.as_deref(), Some("59762-2858"));
		assert_eq!(drug.mpid_version.as_deref(), Some("201411011202"));
		assert_eq!(
			drug.mpid_source_code_system.as_deref(),
			Some("2.16.840.1.113883.6.69")
		);
		assert_eq!(
			drug.mpid_source_code_system_version.as_deref(),
			Some("201411011202")
		);
	}

	#[test]
	fn maps_drug_characterization_by_product_reference() {
		let first_id = "3c91b4d5-e039-4a7a-9c30-67671b0ef9e4";
		let second_id = "4c91b4d5-e039-4a7a-9c30-67671b0ef9e5";
		let xml = format!(
			r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><id root="{first_id}"/><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component><component><substanceAdministration><id root="{second_id}"/><consumable><instanceOfKind><kindOfProduct><name>Drug B</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2><component><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/><subject2><productUseReference><id root="{second_id}"/></productUseReference></subject2></causalityAssessment></component><component><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/><subject2><productUseReference><id root="{first_id}"/></productUseReference></subject2></causalityAssessment></component></MCCI_IN200100UV01>"#
		);

		let drugs = parse_g_drugs(xml.as_bytes()).expect("parse");
		assert_eq!(drugs[0].drug_characterization, "1");
		assert_eq!(drugs[1].drug_characterization, "3");
	}

	#[test]
	fn accepts_non_uuid_product_reference_ids() {
		let xml = r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><id root="d-id1"/><consumable><instanceOfKind><kindOfProduct><name>Drug A</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component><component><substanceAdministration><id root="d-id2"/><consumable><instanceOfKind><kindOfProduct><name>Drug B</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2><component><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/><subject2><productUseReference><id root="d-id2"/></productUseReference></subject2></causalityAssessment></component><component><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/><subject2><productUseReference><id root="d-id1"/></productUseReference></subject2></causalityAssessment></component></MCCI_IN200100UV01>"#;

		let drugs = parse_g_drugs(xml.as_bytes()).expect("parse");
		assert_eq!(drugs[0].xml_id.as_deref(), Some("d-id1"));
		assert_eq!(drugs[1].xml_id.as_deref(), Some("d-id2"));
		assert_eq!(drugs[0].drug_characterization, "1");
		assert_eq!(drugs[1].drug_characterization, "3");
	}

	#[test]
	fn preserves_source_dependent_substance_code_system() {
		let xml = format!(
			r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><id root="{TEST_DRUG_ID}"/><consumable><instanceOfKind><kindOfProduct><name>Drug A</name><ingredient><ingredientSubstance><code code="0Q5GP4Y4FL" codeSystem="2.16.840.1.113883.4.9" codeSystemVersion="12JUL2013"/></ingredientSubstance></ingredient></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2><component><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/><subject2><productUseReference><id root="{TEST_DRUG_ID}"/></productUseReference></subject2></causalityAssessment></component></MCCI_IN200100UV01>"#
		);

		let drugs = parse_g_drugs(xml.as_bytes()).expect("parse");
		let substance = &drugs[0].substances[0];
		assert_eq!(substance.substance_termid.as_deref(), Some("0Q5GP4Y4FL"));
		assert_eq!(
			substance.substance_termid_code_system.as_deref(),
			Some("2.16.840.1.113883.4.9")
		);
		assert_eq!(
			substance.substance_termid_version.as_deref(),
			Some("12JUL2013")
		);
	}

	#[test]
	fn preserves_mfds_product_and_substance_identifiers() {
		let xml = format!(
			r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><id root="{TEST_DRUG_ID}"/><consumable><instanceOfKind><kindOfProduct><code code="200200143" codeSystem="2.16.840.1.113883.3.989.5.1.10.2.1" codeSystemVersion="20240101"/><name>Drug A</name><ingredient><ingredientSubstance><code code="M084105" codeSystem="2.16.840.1.113883.3.989.5.1.10.2.2" codeSystemVersion="20240102"/><name>Substance A</name></ingredientSubstance></ingredient></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2><component><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/><subject2><productUseReference><id root="{TEST_DRUG_ID}"/></productUseReference></subject2></causalityAssessment></component></MCCI_IN200100UV01>"#
		);

		let drugs = parse_g_drugs(xml.as_bytes()).expect("parse");
		assert_eq!(drugs[0].mpid, None);
		assert_eq!(drugs[0].mfds_mpid.as_deref(), Some("200200143"));
		assert_eq!(drugs[0].mfds_mpid_version.as_deref(), Some("20240101"));
		assert_eq!(drugs[0].substances[0].substance_termid, None);
		assert_eq!(drugs[0].substances[0].substance_termid_code_system, None);
		assert_eq!(drugs[0].substances[0].mfds_id.as_deref(), Some("M084105"));
		assert_eq!(
			drugs[0].substances[0].mfds_version.as_deref(),
			Some("20240102")
		);
	}

	#[test]
	fn imports_official_mfds_drug_roles_and_regional_identifiers() {
		let Ok(xml) = std::fs::read(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/../../../docs/exporter/mfds/1-1_ExampleCase_literature_KR_initial_v1_0_샘플.xml"
		)) else {
			return;
		};

		let drugs = parse_g_drugs(&xml).expect("parse official MFDS sample");

		assert_eq!(drugs.len(), 2);
		assert_eq!(drugs[0].drug_characterization, "1");
		assert_eq!(drugs[1].drug_characterization, "3");
		assert_eq!(drugs[0].mfds_mpid.as_deref(), Some("200200143"));
		assert_eq!(drugs[0].substances[0].mfds_id.as_deref(), Some("M084105"));
		assert_eq!(drugs[1].substances[0].mfds_id.as_deref(), Some("M222875"));
	}
}
