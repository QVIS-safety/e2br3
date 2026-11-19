use crate::error::Error;
use crate::import_sections::shared::{
	first_attr, first_text, normalize_code, parse_xml_id_opt,
};
use crate::Result;
use libxml::parser::Parser;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct DrugObservationImport {
	pub(crate) drug_xml_id: Option<String>,
	pub(crate) drug_sequence: i32,
	pub(crate) reaction_xml_id: Option<String>,
	pub(crate) administration_start_interval_value: Option<Decimal>,
	pub(crate) administration_start_interval_unit: Option<String>,
	pub(crate) last_dose_interval_value: Option<Decimal>,
	pub(crate) last_dose_interval_unit: Option<String>,
	pub(crate) reaction_recurred: Option<String>,
	pub(crate) rechallenge_action: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RelatednessImport {
	pub(crate) drug_xml_id: Option<String>,
	pub(crate) reaction_xml_id: Option<String>,
	pub(crate) source_of_assessment: Option<String>,
	pub(crate) method_of_assessment: Option<String>,
	pub(crate) method_of_assessment_kr1: Option<String>,
	pub(crate) result_of_assessment: Option<String>,
	pub(crate) result_of_assessment_kr1: Option<String>,
	pub(crate) result_of_assessment_kr1_null_flavor: Option<String>,
	pub(crate) result_of_assessment_kr2: Option<String>,
}

/// e2b:G.k.9.i.2.r.2.KR.1
fn read_g_k_9_i_2_r_2_kr_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	first_attr(xpath, node, "hl7:causalityAssessment/hl7:methodCode[@codeSystem='2.16.840.1.113883.3.989.5.1.10.1.4']", "code")
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
	match value {
		Some(value) => value,
		None => "",
	}
		.trim()
		.to_ascii_uppercase()
		.replace(['.', '_', '-'], "")
}

pub(crate) fn import_fda_specialized_product_category(
	drug: &super::GDrugImport,
	characteristics: &[super::GDrugDeviceCharacteristicImport],
) -> Option<String> {
	let mut specialized_product_category =
		drug.fda_specialized_product_category.clone();

	for characteristic in characteristics {
		let normalized =
			normalize_characteristic_code(characteristic.code.as_deref());
		let display = match characteristic.code_display_name.as_deref() {
			Some(display) => display,
			None => "",
		}
			.trim()
			.to_ascii_lowercase();
		let code_value = characteristic
			.value_code
			.as_deref()
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(str::to_string);
		if matches!(normalized.as_str(), "FDAGK101" | "C94031")
			&& display == "fda specialized product category"
		{
			specialized_product_category = code_value;
		}
	}
	specialized_product_category
}

fn read_pause_value(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<Decimal> {
	first_attr(xpath, node, "hl7:pauseQuantity", "value")
		.and_then(|v| v.parse().ok())
}

fn read_pause_unit(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	first_attr(xpath, node, "hl7:pauseQuantity", "unit")
}

/// e2b:G.k.9.i.3.1a
fn read_g_k_9_i_3_1a(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<Decimal> {
	read_pause_value(xpath, node)
}

/// e2b:G.k.9.i.3.1b
fn read_g_k_9_i_3_1b(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	read_pause_unit(xpath, node)
}

/// e2b:G.k.9.i.3.2a
fn read_g_k_9_i_3_2a(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<Decimal> {
	read_pause_value(xpath, node)
}

/// e2b:G.k.9.i.3.2b
fn read_g_k_9_i_3_2b(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	read_pause_unit(xpath, node)
}

/// e2b:G.k.9.i.4
fn read_g_k_9_i_4(xpath: &mut Context, node: &libxml::tree::Node) -> Option<String> {
	normalize_code(
		first_attr(xpath, node, "hl7:value", "code"),
		&["1", "2", "3", "4"],
		"drug_reaction_assessments.recurrence_action",
	)
}

/// e2b:G.k.9.i.2.r.1
fn read_g_k_9_i_2_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	first_text(xpath, node, "hl7:causalityAssessment/hl7:author/hl7:assignedEntity/hl7:code/hl7:originalText")
}

/// e2b:G.k.9.i.2.r.2
fn read_g_k_9_i_2_r_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	first_text(
		xpath,
		node,
		"hl7:causalityAssessment/hl7:methodCode/hl7:originalText",
	)
}

/// e2b:G.k.9.i.2.r.3
fn read_g_k_9_i_2_r_3(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	first_text(xpath, node, "hl7:causalityAssessment/hl7:value[not(@code)]")
}

/// e2b:G.k.9.i.2.r.3.KR.1
fn read_g_k_9_i_2_r_3_kr_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	first_attr(xpath, node, "hl7:causalityAssessment/hl7:value[@codeSystem='2.16.840.1.113883.3.989.5.1.10.1.5']", "code")
}

fn read_g_k_9_i_2_r_3_kr_1_null_flavor(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	first_attr(
		xpath,
		node,
		"hl7:causalityAssessment/hl7:value[@codeSystem='2.16.840.1.113883.3.989.5.1.10.1.5' or @nullFlavor]",
		"nullFlavor",
	)
}

/// e2b:G.k.9.i.2.r.3.KR.2
fn read_g_k_9_i_2_r_3_kr_2(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Option<String> {
	first_attr(xpath, node, "hl7:causalityAssessment/hl7:value[@codeSystem='2.16.840.1.113883.3.989.5.1.10.1.6']", "code")
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
			parse_xml_id_opt(first_attr(&mut xpath, &drug_node, "hl7:id", "root"));
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
			String,
			(Option<Decimal>, Option<String>),
		> = HashMap::new();
		let mut last_dose_map: HashMap<String, (Option<Decimal>, Option<String>)> =
			HashMap::new();
		for rel in time_rels {
			let rel_type = rel.get_attribute("typeCode");
			let reaction_id = parse_xml_id_opt(first_attr(
				&mut xpath,
				&rel,
				"hl7:actReference/hl7:id",
				"root",
			));
			if let Some(reaction_id) = reaction_id {
				if matches!(rel_type.as_deref(), Some("SAS")) {
					administration_start_map.insert(
						reaction_id,
						(
							read_g_k_9_i_3_1a(&mut xpath, &rel),
							read_g_k_9_i_3_1b(&mut xpath, &rel),
						),
					);
				} else if matches!(rel_type.as_deref(), Some("SAE")) {
					last_dose_map.insert(
						reaction_id,
						(
							read_g_k_9_i_3_2a(&mut xpath, &rel),
							read_g_k_9_i_3_2b(&mut xpath, &rel),
						),
					);
				}
			}
		}

		for obs in obs_nodes {
			let reaction_recurred = None;
			let reaction_xml_id = parse_xml_id_opt(first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship1[@typeCode='REFR']/hl7:actReference/hl7:id",
				"root",
			));
			let (
				administration_start_interval_value,
				administration_start_interval_unit,
			) = match reaction_xml_id
				.as_ref()
				.and_then(|id| administration_start_map.get(id).cloned())
			{
				Some(interval) => interval,
				None => (None, None),
			};
			let (last_dose_interval_value, last_dose_interval_unit) =
				match reaction_xml_id
					.as_ref()
					.and_then(|id| last_dose_map.get(id).cloned())
				{
					Some(interval) => interval,
					None => (None, None),
				};
			let rechallenge_action = read_g_k_9_i_4(&mut xpath, &obs);
			observations.push(DrugObservationImport {
				drug_xml_id: drug_xml_id.clone(),
				drug_sequence,
				reaction_xml_id,
				administration_start_interval_value,
				administration_start_interval_unit,
				last_dose_interval_value,
				last_dose_interval_unit,
				reaction_recurred,
				rechallenge_action,
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
		let source_of_assessment = read_g_k_9_i_2_r_1(&mut xpath, &node);
		let method_of_assessment = read_g_k_9_i_2_r_2(&mut xpath, &node);
		let method_of_assessment_kr1 = read_g_k_9_i_2_r_2_kr_1(&mut xpath, &node);
		let result_of_assessment = read_g_k_9_i_2_r_3(&mut xpath, &node);
		let result_of_assessment_kr1 = read_g_k_9_i_2_r_3_kr_1(&mut xpath, &node);
		let result_of_assessment_kr1_null_flavor =
			read_g_k_9_i_2_r_3_kr_1_null_flavor(&mut xpath, &node);
		let result_of_assessment_kr2 = read_g_k_9_i_2_r_3_kr_2(&mut xpath, &node);
		let reaction_xml_id = parse_xml_id_opt(first_attr(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:subject1/hl7:adverseEffectReference/hl7:id",
			"root",
		));
		let drug_xml_id = parse_xml_id_opt(first_attr(
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
			method_of_assessment_kr1,
			result_of_assessment,
			result_of_assessment_kr1,
			result_of_assessment_kr1_null_flavor,
			result_of_assessment_kr2,
		});
	}

	Ok(items)
}

#[cfg(test)]
mod tests {
	use super::parse_relatedness_assessments;

	#[test]
	fn mfds_relatedness_code_uses_kr2_field() {
		let xml = br#"<PORR_IN049016UV xmlns="urn:hl7-org:v3"><component><causalityAssessment><code code="39"/><value codeSystem="2.16.840.1.113883.3.989.5.1.10.1.6" code="1"/></causalityAssessment></component></PORR_IN049016UV>"#;
		let rows = parse_relatedness_assessments(xml).expect("parse");
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].result_of_assessment, None);
		assert_eq!(rows[0].result_of_assessment_kr2.as_deref(), Some("1"));
	}

	#[test]
	fn mfds_relatedness_null_flavor_is_separate_from_value() {
		let xml = br#"<PORR_IN049016UV xmlns="urn:hl7-org:v3"><component><causalityAssessment><code code="39"/><value xsi:type="CE" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" nullFlavor="NA"/></causalityAssessment></component></PORR_IN049016UV>"#;
		let rows = parse_relatedness_assessments(xml).expect("parse");
		assert_eq!(rows[0].result_of_assessment_kr1, None);
		assert_eq!(
			rows[0].result_of_assessment_kr1_null_flavor.as_deref(),
			Some("NA")
		);
	}
}
