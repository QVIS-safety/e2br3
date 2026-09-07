use super::*;

pub(crate) async fn export_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<String> {
	let bundle = load_drug_export_bundle(ctx, mm, case_id).await?;
	crate::export::roundtrip::patch_g_drugs_for_authority(
		raw_xml,
		&bundle.drugs,
		&bundle.substances,
		&bundle.dosages,
		&bundle.indications,
		&bundle.characteristics,
		&bundle.devices,
		&bundle.device_codes,
		&bundle.assessments,
		&bundle.relatedness,
		authority,
	)
}

use crate::export::policy::{normalize_gestation_unit, normalize_time_unit};
use lib_core::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation, FdaDeviceCode, FdaDeviceInformation,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use lib_core::regulatory::RegulatoryAuthority;
use sqlx::types::time::{Date, Time};
use std::collections::HashMap;

const FDA_MPID_CODE_SYSTEM: &str = "2.16.840.1.113883.6.69";

fn decimal_text(value: &rust_decimal::Decimal) -> String {
	value.normalize().to_string()
}

pub fn export_g_drugs_xml(
	drugs: &[DrugInformation],
	substances: &[DrugActiveSubstance],
	dosages: &[DosageInformation],
	indications: &[DrugIndication],
	characteristics: &[DrugDeviceCharacteristic],
	assessments: &[DrugReactionAssessment],
	relatedness: &[RelatednessAssessment],
) -> Result<String> {
	let mut subs_by_drug: HashMap<sqlx::types::Uuid, Vec<&DrugActiveSubstance>> =
		HashMap::new();
	for sub in substances {
		subs_by_drug.entry(sub.drug_id).or_default().push(sub);
	}
	let mut dosages_by_drug: HashMap<sqlx::types::Uuid, Vec<&DosageInformation>> =
		HashMap::new();
	for dose in dosages {
		dosages_by_drug.entry(dose.drug_id).or_default().push(dose);
	}
	let mut indications_by_drug: HashMap<sqlx::types::Uuid, Vec<&DrugIndication>> =
		HashMap::new();
	for ind in indications {
		indications_by_drug
			.entry(ind.drug_id)
			.or_default()
			.push(ind);
	}
	let mut characteristics_by_drug: HashMap<
		sqlx::types::Uuid,
		Vec<&DrugDeviceCharacteristic>,
	> = HashMap::new();
	for ch in characteristics {
		characteristics_by_drug
			.entry(ch.drug_id)
			.or_default()
			.push(ch);
	}

	for rows in subs_by_drug.values_mut() {
		rows.sort_by_key(|row| row.sequence_number);
	}
	for rows in dosages_by_drug.values_mut() {
		rows.sort_by_key(|row| row.sequence_number);
	}
	for rows in indications_by_drug.values_mut() {
		rows.sort_by_key(|row| row.sequence_number);
	}
	for rows in characteristics_by_drug.values_mut() {
		rows.sort_by_key(|row| row.sequence_number);
	}

	let mut ordered_drugs: Vec<&DrugInformation> = drugs.iter().collect();
	ordered_drugs.sort_by_key(|drug| drug.sequence_number);

	let mut items_xml = String::new();
	let mut causality_xml = String::new();
	for drug in ordered_drugs {
		let subs = subs_by_drug.get(&drug.id).cloned().unwrap_or_default();
		let doses = dosages_by_drug.get(&drug.id).cloned().unwrap_or_default();
		let inds = indications_by_drug
			.get(&drug.id)
			.cloned()
			.unwrap_or_default();
		let chars = characteristics_by_drug
			.get(&drug.id)
			.cloned()
			.unwrap_or_default();
		let mut drug_assessments: Vec<&DrugReactionAssessment> = assessments
			.iter()
			.filter(|assessment| assessment.drug_id == drug.id)
			.collect();
		drug_assessments.sort_by_key(|assessment| assessment.reaction_id);
		items_xml.push_str(&write_g_k_drug(
			drug,
			&subs,
			&doses,
			&inds,
			&chars,
			&[],
			&[],
			&drug_assessments,
			RegulatoryAuthority::Ich,
		)?);
		causality_xml.push_str(&write_g_k_causality_assessments(
			drug,
			&drug_assessments,
			relatedness,
			RegulatoryAuthority::Ich,
		)?);
	}
	let xml = base_g_drug_skeleton()
		.replace("{DRUGS}", &items_xml)
		.replace("{CAUSALITY}", &causality_xml);
	Ok(xml)
}

/// e2b:G.k.2.2
fn write_g_k_2_2(value: &DrugInformation) -> &str {
	&value.medicinal_product
}

/// e2b:G.k.2.1.1a
fn write_g_k_2_1_1a(value: &DrugInformation) -> Option<&str> {
	value.mpid_version.as_deref()
}

/// e2b:G.k.2.1.1b
fn write_g_k_2_1_1b(value: &DrugInformation) -> Option<&str> {
	value.mpid.as_deref()
}

/// e2b:G.k.2.1.2a
fn write_g_k_2_1_2a(value: &DrugInformation) -> Option<&str> {
	value.phpid_version.as_deref()
}

/// e2b:G.k.2.1.2b
fn write_g_k_2_1_2b(value: &DrugInformation) -> Option<&str> {
	value.phpid.as_deref()
}

/// e2b:G.k.2.1.KR.1a
fn write_g_k_2_1_kr_1a(value: &DrugInformation) -> Option<&str> {
	value.mfds_mpid_version.as_deref()
}

/// e2b:G.k.2.1.KR.1b
fn write_g_k_2_1_kr_1b(value: &DrugInformation) -> Option<&str> {
	value.mfds_mpid.as_deref()
}

/// e2b:G.k.2.4
fn write_g_k_2_4(value: &DrugInformation) -> Option<&str> {
	value.obtain_drug_country.as_deref()
}

/// e2b:G.k.2.5
fn write_g_k_2_5(value: &DrugInformation) -> Option<bool> {
	value.investigational_product_blinded
}

/// e2b:G.k.2.3.r.1
fn write_g_k_2_3_r_1(value: &DrugActiveSubstance) -> Option<&str> {
	value.substance_name.as_deref()
}

/// e2b:G.k.2.3.r.2a
fn write_g_k_2_3_r_2a(value: &DrugActiveSubstance) -> Option<&str> {
	value.substance_termid_version.as_deref()
}

/// e2b:G.k.2.3.r.2b
fn write_g_k_2_3_r_2b(value: &DrugActiveSubstance) -> Option<&str> {
	value.substance_termid.as_deref()
}

/// XML code system metadata for G.k.2.3.r.2.
fn write_g_k_2_3_r_2_code_system(value: &DrugActiveSubstance) -> Option<&str> {
	value.substance_termid_code_system.as_deref()
}

/// e2b:G.k.2.3.r.3a
fn write_g_k_2_3_r_3a(
	value: &DrugActiveSubstance,
) -> Option<&rust_decimal::Decimal> {
	value.strength_value.as_ref()
}

/// e2b:G.k.2.3.r.3b
fn write_g_k_2_3_r_3b(value: &DrugActiveSubstance) -> Option<&str> {
	value.strength_unit.as_deref()
}

/// e2b:G.k.2.3.r.1.KR.1a
fn write_g_k_2_3_r_1_kr_1a(value: &DrugActiveSubstance) -> Option<&str> {
	value.mfds_version.as_deref()
}

/// e2b:G.k.2.3.r.1.KR.1b
fn write_g_k_2_3_r_1_kr_1b(value: &DrugActiveSubstance) -> Option<&str> {
	value.mfds_id.as_deref()
}

/// e2b:G.k.3.1
fn write_g_k_3_1(value: &DrugInformation) -> Option<&str> {
	value.drug_authorization_number.as_deref()
}

/// e2b:G.k.3.2
fn write_g_k_3_2(value: &DrugInformation) -> Option<&str> {
	value.manufacturer_country.as_deref()
}

/// e2b:G.k.3.3
fn write_g_k_3_3(value: &DrugInformation) -> Option<&str> {
	value.manufacturer_name.as_deref()
}

/// e2b:G.k.8
fn write_g_k_8(value: &DrugInformation) -> Option<&str> {
	value.action_taken.as_deref()
}

/// e2b:G.k.5a
fn write_g_k_5a(value: &DrugInformation) -> Option<&rust_decimal::Decimal> {
	value.cumulative_dose_first_reaction_value.as_ref()
}

/// e2b:G.k.5b
fn write_g_k_5b(value: &DrugInformation) -> Option<&str> {
	value.cumulative_dose_first_reaction_unit.as_deref()
}

/// e2b:G.k.6a
fn write_g_k_6a(value: &DrugInformation) -> Option<&rust_decimal::Decimal> {
	value.gestation_period_exposure_value.as_ref()
}

/// e2b:G.k.6b
fn write_g_k_6b(value: &DrugInformation) -> Option<&'static str> {
	normalize_gestation_unit(value.gestation_period_exposure_unit.as_deref()?)
}

/// e2b:G.k.11
fn write_g_k_11(value: &DrugInformation) -> Option<&str> {
	value
		.drug_additional_information
		.as_deref()
		.filter(|v| !v.trim().is_empty())
}

/// e2b:G.k.10.r
fn write_g_k_10_r(value: &DrugInformation) -> Vec<&str> {
	value
		.drug_additional_info_codes_json
		.as_ref()
		.and_then(serde_json::Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(|entry| {
			entry.get("value_code").and_then(serde_json::Value::as_str)
		})
		.filter(|code| !code.trim().is_empty())
		.collect()
}

/// e2b:FDA.G.k.10a
fn write_fda_g_k_10a(value: &DrugInformation) -> Option<&str> {
	value.fda_additional_info_coded.as_deref()
}

/// e2b:FDA.G.k.10.1
fn write_fda_g_k_10_1(
	value: &DrugDeviceCharacteristic,
) -> &DrugDeviceCharacteristic {
	value
}

/// e2b:FDA.G.k.12.r.1
fn write_fda_g_k_12_r_1(value: &FdaDeviceInformation) -> Option<bool> {
	value.malfunction
}

/// e2b:FDA.G.k.12.r.2.r
fn write_fda_g_k_12_r_2_r(value: &FdaDeviceCode) -> Option<&str> {
	(value.element == "follow_up_type").then_some(value.value_code.as_str())
}

/// e2b:FDA.G.k.12.r.3.r
fn write_fda_g_k_12_r_3_r(value: &FdaDeviceCode) -> Option<&str> {
	(value.element == "device_problem").then_some(value.value_code.as_str())
}

/// e2b:FDA.G.k.12.r.4
fn write_fda_g_k_12_r_4(value: &FdaDeviceInformation) -> Option<&str> {
	value.device_brand_name.as_deref()
}

/// e2b:FDA.G.k.12.r.5
fn write_fda_g_k_12_r_5(value: &FdaDeviceInformation) -> Option<&str> {
	value.common_device_name.as_deref()
}

/// e2b:FDA.G.k.12.r.6
fn write_fda_g_k_12_r_6(value: &FdaDeviceInformation) -> Option<&str> {
	value.device_product_code.as_deref()
}

/// e2b:FDA.G.k.12.r.7.1a
fn write_fda_g_k_12_r_7_1a(value: &FdaDeviceInformation) -> Option<&str> {
	value.manufacturer_name.as_deref()
}

/// e2b:FDA.G.k.12.r.7.1b
fn write_fda_g_k_12_r_7_1b(value: &FdaDeviceInformation) -> Option<&str> {
	value.manufacturer_address.as_deref()
}

/// e2b:FDA.G.k.12.r.7.1c
fn write_fda_g_k_12_r_7_1c(value: &FdaDeviceInformation) -> Option<&str> {
	value.manufacturer_city.as_deref()
}

/// e2b:FDA.G.k.12.r.7.1d
fn write_fda_g_k_12_r_7_1d(value: &FdaDeviceInformation) -> Option<&str> {
	value.manufacturer_state.as_deref()
}

/// e2b:FDA.G.k.12.r.7.1e
fn write_fda_g_k_12_r_7_1e(value: &FdaDeviceInformation) -> Option<&str> {
	value.manufacturer_country.as_deref()
}

/// e2b:FDA.G.k.12.r.8
fn write_fda_g_k_12_r_8(value: &FdaDeviceInformation) -> Option<&str> {
	value.device_usage.as_deref()
}

/// e2b:FDA.G.k.12.r.9
fn write_fda_g_k_12_r_9(value: &FdaDeviceInformation) -> Option<&str> {
	value.device_lot_number.as_deref()
}

/// e2b:FDA.G.k.12.r.10
fn write_fda_g_k_12_r_10(value: &FdaDeviceInformation) -> Option<&str> {
	value.operator_of_device.as_deref()
}

/// e2b:FDA.G.k.12.r.11.r
fn write_fda_g_k_12_r_11_r(value: &FdaDeviceCode) -> Option<&str> {
	(value.element == "remedial_action").then_some(value.value_code.as_str())
}

fn write_fda_device_characteristic(
	value: &DrugDeviceCharacteristic,
) -> Option<&DrugDeviceCharacteristic> {
	match value
		.code
		.as_deref()
		.map(str::trim)
		.map(export_characteristic_code)
	{
		Some("FDAGK101") => Some(write_fda_g_k_10_1(value)),
		_ => None,
	}
}

/// e2b:G.k.4.r.1a
fn write_g_k_4_r_1a(value: &DosageInformation) -> Option<&rust_decimal::Decimal> {
	value.dose_value.as_ref()
}

/// e2b:G.k.4.r.1b
fn write_g_k_4_r_1b(value: &DosageInformation) -> Option<&str> {
	value.dose_unit.as_deref()
}

/// e2b:G.k.4.r.2
fn write_g_k_4_r_2(value: &DosageInformation) -> Option<&rust_decimal::Decimal> {
	value.number_of_units.as_ref()
}

/// e2b:G.k.4.r.3
fn write_g_k_4_r_3(value: &DosageInformation) -> Option<&str> {
	value.frequency_unit.as_deref()
}

/// e2b:G.k.4.r.4
fn write_g_k_4_r_4(
	value: &DosageInformation,
) -> (Option<Date>, Option<&str>, Option<&str>) {
	(
		value.first_administration_date,
		value
			.first_administration_date_raw
			.as_deref()
			.and_then(partial_ts_lexeme),
		value.first_administration_date_null_flavor.as_deref(),
	)
}

/// e2b:G.k.4.r.5
fn write_g_k_4_r_5(
	value: &DosageInformation,
) -> (Option<Date>, Option<&str>, Option<&str>) {
	(
		value.last_administration_date,
		value
			.last_administration_date_raw
			.as_deref()
			.and_then(partial_ts_lexeme),
		value.last_administration_date_null_flavor.as_deref(),
	)
}

fn partial_ts_lexeme(value: &str) -> Option<&str> {
	let value = value.trim();
	(matches!(value.len(), 4 | 6) && value.bytes().all(|byte| byte.is_ascii_digit()))
		.then_some(value)
}

/// e2b:G.k.4.r.6a
fn write_g_k_4_r_6a(value: &DosageInformation) -> Option<&rust_decimal::Decimal> {
	value.duration_value.as_ref()
}

/// e2b:G.k.4.r.6b
fn write_g_k_4_r_6b(value: &DosageInformation) -> Option<&'static str> {
	normalize_time_unit(value.duration_unit.as_deref()?)
}

/// e2b:G.k.4.r.7
fn write_g_k_4_r_7(value: &DosageInformation) -> Option<&str> {
	value.batch_lot_number.as_deref()
}

/// e2b:G.k.4.r.8
fn write_g_k_4_r_8(value: &DosageInformation) -> Option<&str> {
	value.dosage_text.as_deref()
}

/// e2b:G.k.4.r.9.1
fn write_g_k_4_r_9_1(value: &DosageInformation) -> Option<&str> {
	value.dose_form.as_deref()
}

/// e2b:G.k.4.r.9.2a
fn write_g_k_4_r_9_2a(value: &DosageInformation) -> Option<&str> {
	value.dose_form_termid_version.as_deref()
}

/// e2b:G.k.4.r.9.2b
fn write_g_k_4_r_9_2b(value: &DosageInformation) -> Option<&str> {
	value.dose_form_termid.as_deref()
}

/// e2b:G.k.4.r.10.1
fn write_g_k_4_r_10_1(value: &DosageInformation) -> Option<&str> {
	value.route_of_administration.as_deref()
}

/// e2b:G.k.4.r.10.2a
fn write_g_k_4_r_10_2a(value: &DosageInformation) -> Option<&str> {
	value.route_termid_version.as_deref()
}

/// e2b:G.k.4.r.10.2b
fn write_g_k_4_r_10_2b(value: &DosageInformation) -> Option<&str> {
	value.route_termid.as_deref()
}

fn write_g_k_4_r_10_2_code_system(value: &DosageInformation) -> Option<&str> {
	value.route_termid_code_system.as_deref()
}

/// e2b:G.k.4.r.11.1
fn write_g_k_4_r_11_1(value: &DosageInformation) -> Option<&str> {
	value.parent_route.as_deref()
}

/// e2b:G.k.4.r.11.2a
fn write_g_k_4_r_11_2a(value: &DosageInformation) -> Option<&str> {
	value.parent_route_termid_version.as_deref()
}

/// e2b:G.k.4.r.11.2b
fn write_g_k_4_r_11_2b(value: &DosageInformation) -> Option<&str> {
	value.parent_route_termid.as_deref()
}

fn write_g_k_4_r_11_2_code_system(value: &DosageInformation) -> Option<&str> {
	value.parent_route_termid_code_system.as_deref()
}

fn write_fda_g_k_12_device_characteristic(
	out: &mut String,
	code: &str,
	code_system: &str,
	display_name: &str,
	value_code: &str,
	value_code_system: &str,
) {
	out.push_str("<subjectOf typeCode=\"SBJ\"><characteristic classCode=\"OBS\" moodCode=\"EVN\"><code code=\"");
	out.push_str(code);
	out.push_str("\" codeSystem=\"");
	out.push_str(code_system);
	out.push_str("\" displayName=\"");
	out.push_str(display_name);
	out.push_str("\"/><value xsi:type=\"CE\" code=\"");
	out.push_str(&xml_escape(value_code));
	out.push_str("\" codeSystem=\"");
	out.push_str(value_code_system);
	out.push_str("\"/></characteristic></subjectOf>");
}

fn write_fda_g_k_12_device(
	device: &FdaDeviceInformation,
	codes: &[&FdaDeviceCode],
) -> String {
	let mut out = String::from(
		"<part classCode=\"PART\"><partProduct classCode=\"DEV\" determinerCode=\"KIND\">",
	);
	if let Some(code) = write_fda_g_k_12_r_6(device) {
		out.push_str("<code code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"");
		if let Some(name) = write_fda_g_k_12_r_5(device) {
			out.push_str(" displayName=\"");
			out.push_str(&xml_escape(name));
			out.push('"');
		}
		out.push_str("/>");
	}
	for (value, null_flavor) in [
		(
			write_fda_g_k_12_r_4(device),
			device.device_brand_name_null_flavor.as_deref(),
		),
		(
			write_fda_g_k_12_r_5(device),
			device.common_device_name_null_flavor.as_deref(),
		),
	] {
		if let Some(value) = value {
			out.push_str("<name>");
			out.push_str(&xml_escape(value));
			out.push_str("</name>");
		} else {
			out.push_str("<name nullFlavor=\"");
			out.push_str(null_flavor.unwrap_or("NI"));
			out.push_str("\"/>");
		}
	}

	let has_manufacturer = write_fda_g_k_12_r_7_1a(device).is_some()
		|| write_fda_g_k_12_r_7_1b(device).is_some()
		|| write_fda_g_k_12_r_7_1c(device).is_some()
		|| write_fda_g_k_12_r_7_1d(device).is_some()
		|| write_fda_g_k_12_r_7_1e(device).is_some();
	let has_characteristics = device.malfunction.is_some()
		|| device.device_usage.is_some()
		|| device.operator_of_device.is_some()
		|| !codes.is_empty();
	if has_manufacturer || has_characteristics {
		out.push_str("<asManufacturedProduct classCode=\"MANU\">");
		if has_manufacturer {
			out.push_str("<manufacturerOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\">");
			if let Some(value) = write_fda_g_k_12_r_7_1a(device) {
				out.push_str("<name>");
				out.push_str(&xml_escape(value));
				out.push_str("</name>");
			}
			if write_fda_g_k_12_r_7_1b(device).is_some()
				|| write_fda_g_k_12_r_7_1c(device).is_some()
				|| write_fda_g_k_12_r_7_1d(device).is_some()
				|| write_fda_g_k_12_r_7_1e(device).is_some()
			{
				out.push_str("<addr>");
				for (tag, value) in [
					("streetAddressLine", write_fda_g_k_12_r_7_1b(device)),
					("city", write_fda_g_k_12_r_7_1c(device)),
					("state", write_fda_g_k_12_r_7_1d(device)),
					("country", write_fda_g_k_12_r_7_1e(device)),
				] {
					if let Some(value) = value {
						out.push('<');
						out.push_str(tag);
						out.push('>');
						out.push_str(&xml_escape(value));
						out.push_str("</");
						out.push_str(tag);
						out.push('>');
					}
				}
				out.push_str("</addr>");
			}
			out.push_str("</manufacturerOrganization>");
		}
		if let Some(value) = write_fda_g_k_12_r_1(device) {
			out.push_str("<subjectOf typeCode=\"SBJ\"><characteristic classCode=\"OBS\" moodCode=\"EVN\"><code code=\"C54026\" codeSystem=\"2.16.840.1.113883.3.26.1.1\" displayName=\"Malfunction\"/><value xsi:type=\"BL\" value=\"");
			out.push_str(if value { "true" } else { "false" });
			out.push_str("\"/></characteristic></subjectOf>");
		}
		for code in codes {
			if let Some(value) = write_fda_g_k_12_r_3_r(code) {
				write_fda_g_k_12_device_characteristic(
					&mut out,
					"C54451",
					"2.16.840.1.113883.3.26.1.1",
					"Device Problem Code",
					value,
					"2.16.840.1.113883.3.26.1.1",
				);
			} else if let Some(value) = write_fda_g_k_12_r_11_r(code) {
				write_fda_g_k_12_device_characteristic(
					&mut out,
					"C54594",
					"2.16.840.1.113883.3.26.1.1",
					"Remedial Action Initiated",
					value,
					"2.16.840.1.113883.3.989.5.1.2.1.1.3",
				);
			} else if let Some(value) = write_fda_g_k_12_r_2_r(code) {
				write_fda_g_k_12_device_characteristic(
					&mut out,
					"C54592",
					"2.16.840.1.113883.3.26.1.1",
					"Type of Follow Up Report",
					value,
					"2.16.840.1.113883.3.989.5.1.2.1.1.5",
				);
			}
		}
		if let Some(value) = write_fda_g_k_12_r_8(device) {
			write_fda_g_k_12_device_characteristic(
				&mut out,
				"C54595",
				"2.16.840.1.113883.3.989.2.1.1.19",
				"Device Usage",
				value,
				"2.16.840.1.113883.3.989.5.1.2.1.1.4",
			);
		}
		if let Some(value) = write_fda_g_k_12_r_10(device) {
			out.push_str("<subjectOf typeCode=\"SBJ\"><characteristic classCode=\"OBS\" moodCode=\"EVN\"><code codeSystem=\"2.16.840.1.113883.3.989.5.1.2.1.1.6\" code=\"");
			out.push_str(&xml_escape(value));
			out.push_str("\"/></characteristic></subjectOf>");
		}
		out.push_str("</asManufacturedProduct>");
	}
	if let Some(value) = write_fda_g_k_12_r_9(device) {
		out.push_str("<instanceOfKind classCode=\"INST\"><productInstanceInstance classCode=\"MMAT\" determinerCode=\"INSTANCE\"><lotNumberText>");
		out.push_str(&xml_escape(value));
		out.push_str("</lotNumberText></productInstanceInstance></instanceOfKind>");
	}
	out.push_str("</partProduct></part>");
	out
}

pub(crate) fn write_g_k_drug(
	drug: &DrugInformation,
	substances: &[&DrugActiveSubstance],
	dosages: &[&DosageInformation],
	indications: &[&DrugIndication],
	characteristics: &[&DrugDeviceCharacteristic],
	devices: &[&FdaDeviceInformation],
	device_codes: &[&FdaDeviceCode],
	assessments: &[&DrugReactionAssessment],
	authority: RegulatoryAuthority,
) -> Result<String> {
	let mut out = String::new();
	let product_name = write_g_k_2_2(drug);

	out.push_str("<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\">");
	out.push_str(
		"<code code=\"4\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/>",
	);
	out.push_str("<component typeCode=\"COMP\"><substanceAdministration classCode=\"SBADM\" moodCode=\"EVN\">");
	out.push_str("<id root=\"");
	out.push_str(&xml_escape(&drug.id.to_string()));
	out.push_str("\"/>");
	out.push_str(
		"<consumable typeCode=\"CSM\"><instanceOfKind classCode=\"INST\"><kindOfProduct classCode=\"MMAT\" determinerCode=\"KIND\">",
	);
	if matches!(authority, RegulatoryAuthority::Mfds)
		&& (write_g_k_2_1_kr_1b(drug).is_some()
			|| write_g_k_2_1_kr_1a(drug).is_some())
	{
		out.push_str("<code codeSystem=\"2.16.840.1.113883.3.989.5.1.10.2.1\"");
		if let Some(code) = write_g_k_2_1_kr_1b(drug) {
			out.push_str(" code=\"");
			out.push_str(&xml_escape(code));
			out.push('"');
		}
		if let Some(version) = write_g_k_2_1_kr_1a(drug) {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(version));
			out.push('"');
		}
		out.push_str("/>");
	} else if matches!(authority, RegulatoryAuthority::Fda)
		&& (write_g_k_2_1_1b(drug).is_some() || write_g_k_2_1_1a(drug).is_some())
	{
		out.push_str("<code codeSystem=\"");
		out.push_str(
			drug.mpid_source_code_system
				.as_deref()
				.unwrap_or(FDA_MPID_CODE_SYSTEM),
		);
		out.push('"');
		if let Some(code) = write_g_k_2_1_1b(drug) {
			out.push_str(" code=\"");
			out.push_str(&xml_escape(code));
			out.push('"');
		}
		if let Some(version) = drug
			.mpid_source_code_system_version
			.as_deref()
			.or_else(|| write_g_k_2_1_1a(drug))
		{
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(version));
			out.push('"');
		}
		out.push_str("/>");
	}
	out.push_str("<name>");
	out.push_str(&xml_escape(product_name));
	out.push_str("</name>");
	if !matches!(authority, RegulatoryAuthority::Fda)
		&& (write_g_k_2_1_1b(drug).is_some() || write_g_k_2_1_1a(drug).is_some())
	{
		out.push_str("<asIdentifiedEntity classCode=\"IDENT\"><id");
		if let Some(mpid) = write_g_k_2_1_1b(drug) {
			out.push_str(" extension=\"");
			out.push_str(&xml_escape(mpid));
			out.push('"');
		}
		out.push_str(
			"/><code code=\"MPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"",
		);
		if let Some(ver) = write_g_k_2_1_1a(drug) {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(ver));
			out.push('"');
		}
		out.push_str("/></asIdentifiedEntity>");
	}
	if write_g_k_2_1_2b(drug).is_some() || write_g_k_2_1_2a(drug).is_some() {
		out.push_str("<asIdentifiedEntity classCode=\"IDENT\"><id");
		if let Some(phpid) = write_g_k_2_1_2b(drug) {
			out.push_str(" extension=\"");
			out.push_str(&xml_escape(phpid));
			out.push('"');
		}
		out.push_str(
			"/><code code=\"PHPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"",
		);
		if let Some(ver) = write_g_k_2_1_2a(drug) {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(ver));
			out.push('"');
		}
		out.push_str("/></asIdentifiedEntity>");
	}
	if drug.manufacturer_name.is_some()
		|| drug.manufacturer_country.is_some()
		|| drug.drug_authorization_number.is_some()
	{
		out.push_str("<asManufacturedProduct classCode=\"MANU\"><subjectOf typeCode=\"SBJ\"><approval classCode=\"CNTRCT\" moodCode=\"EVN\">");
		if let Some(number) = write_g_k_3_1(drug) {
			out.push_str(
				"<id root=\"2.16.840.1.113883.3.989.2.1.3.4\" extension=\"",
			);
			out.push_str(&xml_escape(number));
			out.push_str("\"/>");
		}
		if let Some(name) = write_g_k_3_3(drug) {
			out.push_str("<holder typeCode=\"HLD\"><role classCode=\"HLD\"><playingOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name>");
			out.push_str(&xml_escape(name));
			out.push_str("</name></playingOrganization></role></holder>");
		}
		if let Some(country) = write_g_k_3_2(drug) {
			out.push_str("<author typeCode=\"AUT\"><territorialAuthority classCode=\"TERR\"><territory classCode=\"NAT\" determinerCode=\"INSTANCE\"><code code=\"");
			out.push_str(&xml_escape(country));
			out.push_str("\" codeSystem=\"1.0.3166.1.2.2\"/></territory></territorialAuthority></author>");
		}
		out.push_str("</approval></subjectOf></asManufacturedProduct>");
	}
	if !substances.is_empty() {
		for sub in substances {
			out.push_str("<ingredient>");
			if write_g_k_2_3_r_3a(sub).is_some() || write_g_k_2_3_r_3b(sub).is_some()
			{
				out.push_str("<quantity><numerator");
				if let Some(v) = write_g_k_2_3_r_3a(sub) {
					out.push_str(" value=\"");
					out.push_str(&xml_escape(&decimal_text(v)));
					out.push('"');
				}
				if let Some(u) = write_g_k_2_3_r_3b(sub) {
					out.push_str(" unit=\"");
					out.push_str(&xml_escape(u));
					out.push('"');
				}
				out.push_str("/><denominator value=\"1\" unit=\"1\"/></quantity>");
			}
			out.push_str("<ingredientSubstance>");
			if matches!(authority, RegulatoryAuthority::Mfds)
				&& (write_g_k_2_3_r_1_kr_1b(sub).is_some()
					|| write_g_k_2_3_r_1_kr_1a(sub).is_some())
			{
				out.push_str(
					"<code codeSystem=\"2.16.840.1.113883.3.989.5.1.10.2.2\"",
				);
				if let Some(code) = write_g_k_2_3_r_1_kr_1b(sub) {
					out.push_str(" code=\"");
					out.push_str(&xml_escape(code));
					out.push('"');
				}
				if let Some(version) = write_g_k_2_3_r_1_kr_1a(sub) {
					out.push_str(" codeSystemVersion=\"");
					out.push_str(&xml_escape(version));
					out.push('"');
				}
				out.push_str("/>");
			} else if write_g_k_2_3_r_2b(sub).is_some()
				|| write_g_k_2_3_r_2a(sub).is_some()
				|| write_g_k_2_3_r_2_code_system(sub).is_some()
			{
				out.push_str("<code");
				if let Some(code) = write_g_k_2_3_r_2b(sub) {
					out.push_str(" code=\"");
					out.push_str(&xml_escape(code));
					out.push('"');
				}
				if let Some(code_system) = write_g_k_2_3_r_2_code_system(sub) {
					out.push_str(" codeSystem=\"");
					out.push_str(&xml_escape(code_system));
					out.push('"');
				}
				if let Some(ver) = write_g_k_2_3_r_2a(sub) {
					out.push_str(" codeSystemVersion=\"");
					out.push_str(&xml_escape(ver));
					out.push('"');
				}
				out.push_str("/>");
			}
			if let Some(name) = write_g_k_2_3_r_1(sub) {
				out.push_str("<name>");
				out.push_str(&xml_escape(name));
				out.push_str("</name>");
			}
			out.push_str("</ingredientSubstance>");
			out.push_str("</ingredient>");
		}
	}
	if matches!(authority, RegulatoryAuthority::Fda) && !characteristics.is_empty() {
		for ch in characteristics {
			let Some(ch) = write_fda_device_characteristic(ch) else {
				continue;
			};
			out.push_str("<part><partProduct><asManufacturedProduct><subjectOf><characteristic>");
			let code = ch.code.as_deref().map(str::trim).filter(|v| !v.is_empty());
			let code_system = ch
				.code_system
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let code_display_name = ch
				.code_display_name
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_type = ch
				.value_type
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_value = ch
				.value_value
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_code = ch
				.value_code
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_code_system = ch
				.value_code_system
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_display_name = ch
				.value_display_name
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			out.push_str("<code");
			let use_value_code_as_code =
				code.is_none() && value_value.is_none() && value_code.is_some();
			if let Some(code) = if use_value_code_as_code {
				value_code
			} else {
				code.map(export_characteristic_code)
			} {
				out.push_str(" code=\"");
				out.push_str(&xml_escape(code));
				out.push('"');
			}
			if let Some(cs) = if use_value_code_as_code {
				value_code_system
			} else {
				code_system
			} {
				out.push_str(" codeSystem=\"");
				out.push_str(&xml_escape(cs));
				out.push('"');
			}
			if let Some(name) = if use_value_code_as_code {
				value_display_name
			} else {
				code_display_name
			} {
				out.push_str(" displayName=\"");
				out.push_str(&xml_escape(name));
				out.push('"');
			}
			out.push_str("/>");
			if !use_value_code_as_code {
				out.push_str("<value");
				let normalized_value_type =
					value_type.map(|value| value.to_ascii_uppercase());
				if let Some(vt) = value_type {
					out.push_str(" xsi:type=\"");
					out.push_str(&xml_escape(vt));
					out.push('"');
				}
				let renders_text_body = matches!(
					normalized_value_type.as_deref(),
					Some("ST") | Some("ED")
				);
				if let Some(v) = value_value.filter(|_| !renders_text_body) {
					out.push_str(" value=\"");
					out.push_str(&xml_escape(v));
					out.push('"');
				}
				if let Some(code) = value_code {
					out.push_str(" code=\"");
					out.push_str(&xml_escape(code));
					out.push('"');
				}
				if let Some(cs) = value_code_system {
					out.push_str(" codeSystem=\"");
					out.push_str(&xml_escape(cs));
					out.push('"');
				}
				if let Some(name) = value_display_name {
					out.push_str(" displayName=\"");
					out.push_str(&xml_escape(name));
					out.push('"');
				}
				if let Some(v) = value_value.filter(|_| renders_text_body) {
					out.push('>');
					out.push_str(&xml_escape(v));
					out.push_str("</value>");
				} else {
					out.push_str("/>");
				}
			}
			out.push_str("</characteristic></subjectOf></asManufacturedProduct></partProduct></part>");
		}
	}
	if let Some(batch) = drug.batch_lot_number.as_deref() {
		out.push_str("<part><partProduct><instanceOfKind><productInstanceInstance><lotNumberText>");
		out.push_str(&xml_escape(batch));
		out.push_str("</lotNumberText></productInstanceInstance></instanceOfKind></partProduct></part>");
	}
	if matches!(authority, RegulatoryAuthority::Fda) {
		for device in devices {
			let codes: Vec<_> = device_codes
				.iter()
				.copied()
				.filter(|code| code.device_id == device.id)
				.collect();
			out.push_str(&write_fda_g_k_12_device(device, &codes));
		}
	}
	out.push_str("</kindOfProduct>");
	if let Some(country) = write_g_k_2_4(drug) {
		out.push_str("<subjectOf typeCode=\"SBJ\"><productEvent classCode=\"ACT\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.18\" displayName=\"retailSupply\"/><performer typeCode=\"PRF\"><assignedEntity classCode=\"ASSIGNED\"><representedOrganization determinerCode=\"INSTANCE\" classCode=\"ORG\"><addr><country>");
		out.push_str(&xml_escape(country));
		out.push_str("</country></addr></representedOrganization></assignedEntity></performer></productEvent></subjectOf>");
	}
	out.push_str("</instanceOfKind></consumable>");
	for assessment in assessments {
		out.push_str(&write_g_k_9_i_time_intervals(assessment));
	}
	for assessment in assessments {
		out.push_str(&write_g_k_9_i_recurrence(assessment));
	}
	for code in write_g_k_10_r(drug) {
		out.push_str("<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"9\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.17\" code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\"/></observation></outboundRelationship2>");
	}
	if drug.cumulative_dose_first_reaction_value.is_some()
		|| drug.cumulative_dose_first_reaction_unit.is_some()
	{
		out.push_str("<outboundRelationship2 typeCode=\"SUMM\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"14\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"cumulativeDoseToReaction\"/><value xsi:type=\"PQ\"");
		if let Some(v) = write_g_k_5a(drug) {
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&decimal_text(v)));
			out.push('"');
		}
		if let Some(u) = write_g_k_5b(drug) {
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(u));
			out.push('"');
		}
		out.push_str("/></observation></outboundRelationship2>");
	}
	if drug.gestation_period_exposure_value.is_some()
		|| drug.gestation_period_exposure_unit.is_some()
	{
		out.push_str("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"16\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"gestationPeriod\"/><value xsi:type=\"PQ\"");
		if let Some(v) = write_g_k_6a(drug) {
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&decimal_text(v)));
			out.push('"');
		}
		if let Some(u) = write_g_k_6b(drug) {
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(u));
			out.push('"');
		}
		out.push_str("/></observation></outboundRelationship2>");
	}
	if matches!(authority, RegulatoryAuthority::Fda) {
		if let Some(code) = write_fda_g_k_10a(drug) {
			out.push_str("<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"9\"/><value xsi:type=\"CE\" code=\"");
			out.push_str(&xml_escape(code));
			out.push_str("\"/></observation></outboundRelationship2>");
		} else if let Some(null_flavor) =
			drug.fda_additional_info_coded_null_flavor.as_deref()
		{
			out.push_str("<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"9\"/><value xsi:type=\"CE\" nullFlavor=\"");
			out.push_str(&xml_escape(null_flavor));
			out.push_str("\"/></observation></outboundRelationship2>");
		}
	}
	if let Some(text) = write_g_k_11(drug) {
		out.push_str("<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ST\">");
		out.push_str(&xml_escape(text));
		out.push_str("</value></observation></outboundRelationship2>");
	}
	for dose in dosages {
		out.push_str("<outboundRelationship2 typeCode=\"COMP\"><substanceAdministration classCode=\"SBADM\" moodCode=\"EVN\">");
		if let Some(text) = write_g_k_4_r_8(dose) {
			out.push_str("<text>");
			out.push_str(&xml_escape(text));
			out.push_str("</text>");
		}
		let has_frequency =
			dose.number_of_units.is_some() || dose.frequency_unit.is_some();
		let (start, start_raw, start_null_flavor) = write_g_k_4_r_4(dose);
		let (end, end_raw, end_null_flavor) = write_g_k_4_r_5(dose);
		let has_bounds = start.is_some()
			|| start_raw.is_some()
			|| start_null_flavor.is_some()
			|| end.is_some()
			|| end_raw.is_some()
			|| end_null_flavor.is_some();
		let has_duration = dose.duration_value.is_some();
		let timing_parts = usize::from(has_frequency)
			+ usize::from(has_bounds)
			+ usize::from(has_duration);
		if timing_parts > 1 {
			out.push_str("<effectiveTime xsi:type=\"SXPR_TS\">");
		}
		if has_frequency {
			out.push_str(if timing_parts > 1 {
				"<comp xsi:type=\"PIVL_TS\"><period"
			} else {
				"<effectiveTime xsi:type=\"PIVL_TS\"><period"
			});
			if let Some(v) = write_g_k_4_r_2(dose) {
				out.push_str(" value=\"");
				out.push_str(&xml_escape(&decimal_text(v)));
				out.push('"');
			}
			if let Some(u) = write_g_k_4_r_3(dose) {
				out.push_str(" unit=\"");
				out.push_str(&xml_escape(u));
				out.push('"');
			}
			out.push_str(if timing_parts > 1 {
				"/></comp>"
			} else {
				"/></effectiveTime>"
			});
		}
		if has_bounds {
			out.push_str(if timing_parts > 1 {
				"<comp xsi:type=\"IVL_TS\" operator=\"A\">"
			} else {
				"<effectiveTime xsi:type=\"IVL_TS\">"
			});
			if let Some(start) = start_raw {
				out.push_str("<low value=\"");
				out.push_str(&xml_escape(start));
				out.push_str("\"/>");
			} else if let Some(start) = start {
				out.push_str("<low value=\"");
				out.push_str(&fmt_ts(start, None));
				out.push_str("\"/>");
			} else if let Some(null_flavor) = start_null_flavor {
				out.push_str("<low nullFlavor=\"");
				out.push_str(&xml_escape(null_flavor));
				out.push_str("\"/>");
			}
			if let Some(end) = end_raw {
				out.push_str("<high value=\"");
				out.push_str(&xml_escape(end));
				out.push_str("\"/>");
			} else if let Some(end) = end {
				out.push_str("<high value=\"");
				out.push_str(&fmt_ts(end, None));
				out.push_str("\"/>");
			} else if let Some(null_flavor) = end_null_flavor {
				out.push_str("<high nullFlavor=\"");
				out.push_str(&xml_escape(null_flavor));
				out.push_str("\"/>");
			}
			out.push_str(if timing_parts > 1 {
				"</comp>"
			} else {
				"</effectiveTime>"
			});
		}
		if has_duration {
			out.push_str(if timing_parts > 1 {
				"<comp xsi:type=\"IVL_TS\" operator=\"A\"><width value=\""
			} else {
				"<effectiveTime xsi:type=\"IVL_TS\"><width value=\""
			});
			if let Some(width) = write_g_k_4_r_6a(dose) {
				out.push_str(&xml_escape(&decimal_text(width)));
				out.push('"');
				if let Some(unit) = write_g_k_4_r_6b(dose) {
					out.push_str(" unit=\"");
					out.push_str(&xml_escape(unit));
					out.push('"');
				}
				out.push_str(if timing_parts > 1 {
					"/></comp>"
				} else {
					"/></effectiveTime>"
				});
			}
		}
		if timing_parts > 1 {
			out.push_str("</effectiveTime>");
		}
		if dose.route_of_administration.is_some()
			|| dose.route_termid.is_some()
			|| dose.route_termid_code_system.is_some()
			|| dose.route_of_administration_null_flavor.is_some()
		{
			out.push_str("<routeCode");
			if let Some(null_flavor) =
				dose.route_of_administration_null_flavor.as_deref()
			{
				out.push_str(" nullFlavor=\"");
				out.push_str(&xml_escape(null_flavor));
				out.push('"');
			} else if let Some(code) = write_g_k_4_r_10_2b(dose) {
				out.push_str(" code=\"");
				out.push_str(&xml_escape(code));
				out.push('"');
			}
			if dose.route_of_administration_null_flavor.is_none() {
				if let Some(code_system) = write_g_k_4_r_10_2_code_system(dose) {
					out.push_str(" codeSystem=\"");
					out.push_str(&xml_escape(code_system));
					out.push('"');
				}
				if let Some(ver) = write_g_k_4_r_10_2a(dose) {
					out.push_str(" codeSystemVersion=\"");
					out.push_str(&xml_escape(ver));
					out.push('"');
				}
			}
			if let Some(route) = write_g_k_4_r_10_1(dose) {
				out.push_str("><originalText>");
				out.push_str(&xml_escape(route));
				out.push_str("</originalText></routeCode>");
			} else {
				out.push_str("/>");
			}
		}
		if dose.dose_value.is_some() || dose.dose_unit.is_some() {
			out.push_str("<doseQuantity");
			if let Some(v) = write_g_k_4_r_1a(dose) {
				out.push_str(" value=\"");
				out.push_str(&xml_escape(&decimal_text(v)));
				out.push('"');
			}
			if let Some(u) = write_g_k_4_r_1b(dose) {
				out.push_str(" unit=\"");
				out.push_str(&xml_escape(u));
				out.push('"');
			}
			out.push_str("/>");
		}
		if dose.batch_lot_number.is_some()
			|| dose.batch_lot_number_null_flavor.is_some()
			|| dose.dose_form.is_some()
			|| dose.dose_form_termid.is_some()
			|| dose.dose_form_null_flavor.is_some()
		{
			out.push_str("<consumable><instanceOfKind>");
			if let Some(batch) = write_g_k_4_r_7(dose) {
				out.push_str("<productInstanceInstance><lotNumberText>");
				out.push_str(&xml_escape(batch));
				out.push_str("</lotNumberText></productInstanceInstance>");
			} else if let Some(null_flavor) =
				dose.batch_lot_number_null_flavor.as_deref()
			{
				out.push_str("<productInstanceInstance><id nullFlavor=\"NI\"/><lotNumberText nullFlavor=\"");
				out.push_str(&xml_escape(null_flavor));
				out.push_str("\"/></productInstanceInstance>");
			}
			if dose.dose_form.is_some()
				|| dose.dose_form_termid.is_some()
				|| dose.dose_form_null_flavor.is_some()
			{
				out.push_str("<kindOfProduct><formCode");
				if let Some(null_flavor) = dose.dose_form_null_flavor.as_deref() {
					out.push_str(" nullFlavor=\"");
					out.push_str(&xml_escape(null_flavor));
					out.push('"');
				} else if let Some(code) = write_g_k_4_r_9_2b(dose) {
					out.push_str(" code=\"");
					out.push_str(&xml_escape(code));
					out.push('"');
				}
				if dose.dose_form_null_flavor.is_none() {
					if let Some(ver) = write_g_k_4_r_9_2a(dose) {
						out.push_str(" codeSystemVersion=\"");
						out.push_str(&xml_escape(ver));
						out.push('"');
					}
				}
				out.push('>');
				if dose.dose_form_null_flavor.is_none() {
					if let Some(text) = write_g_k_4_r_9_1(dose) {
						out.push_str("<originalText>");
						out.push_str(&xml_escape(text));
						out.push_str("</originalText>");
					}
				}
				out.push_str("</formCode></kindOfProduct>");
			}
			out.push_str("</instanceOfKind></consumable>");
		}
		if dose.parent_route_termid.is_some()
			|| dose.parent_route_termid_code_system.is_some()
			|| dose.parent_route.is_some()
			|| dose.parent_route_null_flavor.is_some()
		{
			out.push_str("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"G.k.4.r.11\"/><value");
			out.push_str(" xsi:type=\"CE\"");
			if let Some(null_flavor) = dose.parent_route_null_flavor.as_deref() {
				out.push_str(" nullFlavor=\"");
				out.push_str(&xml_escape(null_flavor));
				out.push('"');
			} else if let Some(code) = write_g_k_4_r_11_2b(dose) {
				out.push_str(" code=\"");
				out.push_str(&xml_escape(code));
				out.push('"');
			}
			if dose.parent_route_null_flavor.is_none() {
				if let Some(code_system) = write_g_k_4_r_11_2_code_system(dose) {
					out.push_str(" codeSystem=\"");
					out.push_str(&xml_escape(code_system));
					out.push('"');
				}
				if let Some(ver) = write_g_k_4_r_11_2a(dose) {
					out.push_str(" codeSystemVersion=\"");
					out.push_str(&xml_escape(ver));
					out.push('"');
				}
			}
			out.push_str("><originalText>");
			if dose.parent_route_null_flavor.is_none() {
				if let Some(text) = write_g_k_4_r_11_1(dose) {
					out.push_str(&xml_escape(text));
				}
			}
			out.push_str(
				"</originalText></value></observation></outboundRelationship2>",
			);
		}
		out.push_str("</substanceAdministration></outboundRelationship2>");
	}
	if write_g_k_2_5(drug) == Some(true) {
		out.push_str("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"6\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"blinded\"/><value xsi:type=\"BL\" value=\"true\"/></observation></outboundRelationship2>");
	}
	if let Some(action) = write_g_k_8(drug) {
		out.push_str("<inboundRelationship typeCode=\"CAUS\"><act classCode=\"ACT\" moodCode=\"EVN\"><code code=\"");
		out.push_str(&xml_escape(action));
		out.push_str("\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.15\"/></act></inboundRelationship>");
	}
	for ind in indications {
		out.push_str("<inboundRelationship typeCode=\"RSON\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"19\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"indication\"/><value xsi:type=\"CE\"");
		if let Some(code) = write_g_k_7_r_2b(ind) {
			out.push_str(" code=\"");
			out.push_str(&xml_escape(code));
			out.push_str("\" codeSystem=\"2.16.840.1.113883.6.163\"");
		}
		if let Some(ver) = write_g_k_7_r_2a(ind) {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(ver));
			out.push('"');
		}
		out.push('>');
		let (text, text_null_flavor) = write_g_k_7_r_1(ind);
		if let Some(text) = text {
			out.push_str("<originalText>");
			out.push_str(&xml_escape(text));
			out.push_str("</originalText>");
		} else if let Some(null_flavor) = text_null_flavor {
			out.push_str("<originalText nullFlavor=\"");
			out.push_str(&xml_escape(null_flavor));
			out.push_str("\"/>");
		}
		out.push_str("</value><performer typeCode=\"PRF\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\" displayName=\"sourceReporter\"/></assignedEntity></performer></observation></inboundRelationship>");
	}
	out.push_str("</substanceAdministration></component></organizer></subjectOf2>");
	Ok(out)
}

/// e2b:G.k.7.r.1
fn write_g_k_7_r_1(value: &DrugIndication) -> (Option<&str>, Option<&str>) {
	(
		value.indication_text.as_deref(),
		value.indication_text_null_flavor.as_deref(),
	)
}

/// e2b:G.k.7.r.2a
fn write_g_k_7_r_2a(value: &DrugIndication) -> Option<&str> {
	value.indication_meddra_version.as_deref()
}

/// e2b:G.k.7.r.2b
fn write_g_k_7_r_2b(value: &DrugIndication) -> Option<&str> {
	value.indication_meddra_code.as_deref()
}

/// e2b:G.k.9.i.3.1a
fn write_g_k_9_i_3_1a(
	value: &DrugReactionAssessment,
) -> Option<&rust_decimal::Decimal> {
	value.administration_start_interval_value.as_ref()
}

/// e2b:G.k.9.i.3.1b
fn write_g_k_9_i_3_1b(value: &DrugReactionAssessment) -> Option<&str> {
	value.administration_start_interval_unit.as_deref()
}

/// e2b:G.k.9.i.3.2a
fn write_g_k_9_i_3_2a(
	value: &DrugReactionAssessment,
) -> Option<&rust_decimal::Decimal> {
	value.last_dose_interval_value.as_ref()
}

/// e2b:G.k.9.i.3.2b
fn write_g_k_9_i_3_2b(value: &DrugReactionAssessment) -> Option<&str> {
	value.last_dose_interval_unit.as_deref()
}

/// e2b:G.k.9.i.4
fn write_g_k_9_i_4(value: &DrugReactionAssessment) -> Option<&str> {
	value.recurrence_action.as_deref()
}

fn write_g_k_9_i_time_intervals(assessment: &DrugReactionAssessment) -> String {
	let mut out = String::new();
	if assessment.administration_start_interval_value.is_some()
		|| assessment.administration_start_interval_unit.is_some()
	{
		out.push_str("<outboundRelationship1 typeCode=\"SAS\"><pauseQuantity");
		if let Some(value) = write_g_k_9_i_3_1a(assessment) {
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&decimal_text(value)));
			out.push('"');
		}
		if let Some(unit) = write_g_k_9_i_3_1b(assessment) {
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(unit));
			out.push('"');
		}
		out.push_str(
			"/><actReference classCode=\"ACT\" moodCode=\"EVN\"><id root=\"",
		);
		out.push_str(&xml_escape(&assessment.reaction_id.to_string()));
		out.push_str("\"/></actReference></outboundRelationship1>");
	}
	if assessment.last_dose_interval_value.is_some()
		|| assessment.last_dose_interval_unit.is_some()
	{
		out.push_str("<outboundRelationship1 typeCode=\"SAE\"><pauseQuantity");
		if let Some(value) = write_g_k_9_i_3_2a(assessment) {
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&decimal_text(value)));
			out.push('"');
		}
		if let Some(unit) = write_g_k_9_i_3_2b(assessment) {
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(unit));
			out.push('"');
		}
		out.push_str(
			"/><actReference classCode=\"ACT\" moodCode=\"EVN\"><id root=\"",
		);
		out.push_str(&xml_escape(&assessment.reaction_id.to_string()));
		out.push_str("\"/></actReference></outboundRelationship1>");
	}
	out
}

fn write_g_k_9_i_recurrence(assessment: &DrugReactionAssessment) -> String {
	let mut out = String::new();
	// G.k.9.i.4 - Did Reaction Recur on Re-administration? (standard single observation,
	// HL7 observation code 31). The recurrence answer uses the official 1-4 enum, which the
	// backend stores in recurrence_action (matching the FDA reference instance value codes).
	// The previously-emitted G.k.8.r.1 / G.k.8.r.2 sub-observations used codes that do not
	// exist in the ICH/FDA/MFDS standards and have been removed.
	if let Some(code) = write_g_k_9_i_4(assessment) {
		out.push_str(
			"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"31\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/>",
		);
		out.push_str("<value xsi:type=\"CE\" code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\"/>");
		out.push_str(
			"<outboundRelationship1 typeCode=\"REFR\"><actReference classCode=\"ACT\" moodCode=\"EVN\"><id root=\"",
		);
		out.push_str(&xml_escape(&assessment.reaction_id.to_string()));
		out.push_str("\"/></actReference></outboundRelationship1>");
		out.push_str("</observation></outboundRelationship2>");
	}
	out
}

pub(crate) fn write_g_k_causality_assessments(
	drug: &DrugInformation,
	assessments: &[&DrugReactionAssessment],
	relatedness: &[RelatednessAssessment],
	authority: RegulatoryAuthority,
) -> Result<String> {
	let mut out = String::new();
	out.push_str(&write_g_k_1_causality(drug));
	if matches!(authority, RegulatoryAuthority::Fda) {
		out.push_str(&write_fda_g_k_1_a_causality(drug));
	}
	for assessment in assessments {
		let mut rows: Vec<&RelatednessAssessment> = relatedness
			.iter()
			.filter(|row| row.drug_reaction_assessment_id == assessment.id)
			.collect();
		rows.sort_by_key(|row| row.sequence_number);
		for row in rows {
			out.push_str(&write_g_k_9_causality(
				drug.id, assessment, row, authority,
			));
		}
	}
	Ok(out)
}

/// e2b:FDA.G.k.1.a
fn write_fda_g_k_1_a(value: &DrugInformation) -> Option<&str> {
	value.fda_other_characterization.as_deref()
}

pub(crate) fn write_fda_g_k_1_a_causality(drug: &DrugInformation) -> String {
	let Some(role) = write_fda_g_k_1_a(drug) else {
		return String::new();
	};
	format!(
		"<component typeCode=\"COMP\"><causalityAssessment classCode=\"OBS\" moodCode=\"EVN\"><code code=\"20\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"interventionCharacterization\"/><value xsi:type=\"CE\" code=\"{}\" displayName=\"Similar Device\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.1.1.8\"/><subject2 typeCode=\"SUBJ\"><productUseReference classCode=\"SBADM\" moodCode=\"EVN\"><id root=\"{}\"/></productUseReference></subject2></causalityAssessment></component>",
		xml_escape(role),
		drug.id
	)
}

pub(crate) fn write_g_k_9_causality(
	drug_id: sqlx::types::Uuid,
	assessment: &DrugReactionAssessment,
	relatedness: &RelatednessAssessment,
	authority: RegulatoryAuthority,
) -> String {
	let mut out = String::new();
	out.push_str("<component typeCode=\"COMP\"><causalityAssessment classCode=\"OBS\" moodCode=\"EVN\">");
	out.push_str("<code code=\"39\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"causality\"/>");
	let mfds_result = matches!(authority, RegulatoryAuthority::Mfds)
		.then(|| write_g_k_9_i_2_r_3_kr_2(relatedness))
		.flatten();
	if let Some(result) = mfds_result {
		out.push_str("<value xsi:type=\"CE\" codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.6\" code=\"");
		out.push_str(&xml_escape(result));
		out.push_str("\"/>");
	} else if matches!(authority, RegulatoryAuthority::Mfds) {
		if let Some(null_flavor) =
			relatedness.result_of_assessment_kr1_null_flavor.as_deref()
		{
			out.push_str("<value xsi:type=\"CE\" nullFlavor=\"");
			out.push_str(&xml_escape(null_flavor));
			out.push_str("\"/>");
		} else if let Some(result) = write_g_k_9_i_2_r_3_kr_1(relatedness) {
			out.push_str("<value xsi:type=\"CE\" codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.5\" code=\"");
			out.push_str(&xml_escape(result));
			out.push_str("\"/>");
		}
	} else if let Some(result) = write_g_k_9_i_2_r_3(relatedness) {
		out.push_str("<value xsi:type=\"ST\">");
		out.push_str(&xml_escape(result));
		out.push_str("</value>");
	}
	if let Some(method) = matches!(authority, RegulatoryAuthority::Mfds)
		.then(|| write_g_k_9_i_2_r_2_kr_1(relatedness))
		.flatten()
	{
		out.push_str(
			"<methodCode codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.4\" code=\"",
		);
		out.push_str(&xml_escape(method));
		out.push_str("\"/>");
	} else if let Some(method) = write_g_k_9_i_2_r_2(relatedness) {
		out.push_str("<methodCode><originalText>");
		out.push_str(&xml_escape(method));
		out.push_str("</originalText></methodCode>");
	}
	if let Some(source) = write_g_k_9_i_2_r_1(relatedness) {
		out.push_str("<author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code><originalText>");
		out.push_str(&xml_escape(source));
		out.push_str("</originalText></code></assignedEntity></author>");
	}
	out.push_str("<subject1 typeCode=\"SUBJ\"><adverseEffectReference classCode=\"OBS\" moodCode=\"EVN\"><id root=\"");
	out.push_str(&xml_escape(&write_g_k_9_i_1(assessment)));
	out.push_str("\"/></adverseEffectReference></subject1>");
	out.push_str("<subject2 typeCode=\"SUBJ\"><productUseReference classCode=\"SBADM\" moodCode=\"EVN\"><id root=\"");
	out.push_str(&xml_escape(&drug_id.to_string()));
	out.push_str("\"/></productUseReference></subject2>");
	out.push_str("</causalityAssessment></component>");
	out
}

/// e2b:G.k.9.i.2.r.1
fn write_g_k_9_i_2_r_1(value: &RelatednessAssessment) -> Option<&str> {
	value.source_of_assessment.as_deref()
}

/// e2b:G.k.9.i.2.r.2
fn write_g_k_9_i_2_r_2(value: &RelatednessAssessment) -> Option<&str> {
	value.method_of_assessment.as_deref()
}

/// e2b:G.k.9.i.2.r.2.KR.1
fn write_g_k_9_i_2_r_2_kr_1(value: &RelatednessAssessment) -> Option<&str> {
	value.method_of_assessment_kr1.as_deref()
}

/// e2b:G.k.9.i.2.r.3
fn write_g_k_9_i_2_r_3(value: &RelatednessAssessment) -> Option<&str> {
	value.result_of_assessment.as_deref()
}

/// e2b:G.k.9.i.2.r.3.KR.1
fn write_g_k_9_i_2_r_3_kr_1(value: &RelatednessAssessment) -> Option<&str> {
	value.result_of_assessment_kr1.as_deref()
}

/// e2b:G.k.9.i.2.r.3.KR.2
fn write_g_k_9_i_2_r_3_kr_2(value: &RelatednessAssessment) -> Option<&str> {
	value.result_of_assessment_kr2.as_deref()
}

/// e2b:G.k.9.i.1
fn write_g_k_9_i_1(value: &DrugReactionAssessment) -> String {
	value.reaction_id.to_string()
}

pub(crate) fn write_g_k_1_causality(drug: &DrugInformation) -> String {
	let code = drug.drug_characterization.trim();
	if code.is_empty() {
		return String::new();
	}
	let display_name = write_g_k_1(drug)
		.map(|(_, name)| format!(" displayName=\"{}\"", xml_escape(name)))
		.unwrap_or_default();
	format!(
		"<component typeCode=\"COMP\"><causalityAssessment classCode=\"OBS\" moodCode=\"EVN\"><code code=\"20\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"interventionCharacterization\"/><value xsi:type=\"CE\" code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.13\"{display_name}/><subject2 typeCode=\"SUBJ\"><productUseReference classCode=\"SBADM\" moodCode=\"EVN\"><id root=\"{drug_id}\"/></productUseReference></subject2></causalityAssessment></component>",
		xml_escape(code),
		drug_id = drug.id,
	)
}

/// e2b:G.k.1
fn write_g_k_1(value: &DrugInformation) -> Option<(&str, &'static str)> {
	match value.drug_characterization.trim() {
		"1" => Some(("1", "Suspect")),
		"2" => Some(("2", "Concomitant")),
		"3" => Some(("3", "Interacting")),
		"4" => Some(("4", "Drug Not Administered")),
		_ => None,
	}
}

fn fmt_date(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

fn fmt_time(time: Time) -> String {
	format!("{:02}{:02}{:02}", time.hour(), time.minute(), time.second())
}

fn fmt_ts(date: Date, time: Option<Time>) -> String {
	let mut out = fmt_date(date);
	if let Some(t) = time {
		out.push_str(&fmt_time(t));
	}
	out
}

fn export_characteristic_code(code: &str) -> &str {
	match code {
		"FDA.G.k.10.1" => "FDAGK101",
		"FDA.G.k.12.r.1" => "FDAGK12R1",
		"FDA.G.k.12.r.2.r" => "FDAGK12R2R",
		"FDA.G.k.12.r.3.r" => "FDAGK12R3R",
		"FDA.G.k.12.r.4" => "FDAGK12R4",
		"FDA.G.k.12.r.5" => "FDAGK12R5",
		"FDA.G.k.12.r.6" => "FDAGK12R6",
		"FDA.G.k.12.r.7.1a" => "FDAGK12R71A",
		"FDA.G.k.12.r.7.1b" => "FDAGK12R71B",
		"FDA.G.k.12.r.7.1c" => "FDAGK12R71C",
		"FDA.G.k.12.r.7.1d" => "FDAGK12R71D",
		"FDA.G.k.12.r.7.1e" => "FDAGK12R71E",
		"FDA.G.k.12.r.8" => "FDAGK12R8",
		"FDA.G.k.12.r.9" => "FDAGK12R9",
		"FDA.G.k.12.r.10" => "FDAGK12R10",
		"FDA.G.k.12.r.11.r" => "FDAGK12R11R",
		other => other,
	}
}

fn base_g_drug_skeleton() -> &'static str {
	"<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ITSVersion=\"XML_1.0\">\
\t<PORR_IN049016UV>\
\t\t<controlActProcess classCode=\"CACT\" moodCode=\"EVN\">\
\t\t\t<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>\
\t\t\t<subject>\
\t\t\t\t<investigationEvent classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t<component typeCode=\"COMP\">\
\t\t\t\t\t\t<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t\t\t<subject1 typeCode=\"SBJ\">\
\t\t\t\t\t\t\t\t<primaryRole classCode=\"INVSBJ\">\
\t\t\t\t\t\t\t\t\t<player1 classCode=\"PSN\" determinerCode=\"INSTANCE\"><name/></player1>\
\t\t\t\t\t\t\t\t\t{DRUGS}\
\t\t\t\t\t\t\t\t</primaryRole>\
\t\t\t\t\t\t\t</subject1>\
\t\t\t\t\t\t\t{CAUSALITY}\
\t\t\t\t\t\t</adverseEventAssessment>\
\t\t\t\t\t</component>\
\t\t\t\t</investigationEvent>\
\t\t\t</subject>\
\t\t</controlActProcess>\
\t</PORR_IN049016UV>\
</MCCI_IN200100UV01>"
}

#[cfg(test)]
mod tests {
	use super::*;
	use rust_decimal::Decimal;
	use sqlx::types::time::Date;
	use sqlx::types::Uuid;
	use std::collections::BTreeSet;
	use time::OffsetDateTime;

	#[test]
	fn section_g_writers_cover_registry_fields() {
		let registry: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../../../registry/sections/g-drug.json"
		))
		.expect("section G registry");
		let expected = registry
			.as_array()
			.expect("registry array")
			.iter()
			.filter(|entry| entry["local_only"] != true)
			.filter_map(|entry| entry["e2br3_code"].as_str())
			.collect::<BTreeSet<_>>();
		let source = format!(
			"{}\n{}",
			include_str!("g.rs"),
			include_str!("../roundtrip/g_drug.rs")
		);
		let implemented = source
			.lines()
			.filter_map(|line| line.trim().strip_prefix("/// e2b:"))
			.collect::<BTreeSet<_>>();

		assert_eq!(implemented, expected);
	}

	fn test_drug(id: Uuid, case_id: Uuid) -> DrugInformation {
		DrugInformation {
			id,
			case_id,
			source_product_presave_id: None,
			sequence_number: 1,
			deleted: false,
			drug_characterization: "1".to_string(),
			medicinal_product: "Drug A".to_string(),
			mpid: Some("BASE-MPID".to_string()),
			mpid_version: Some("BASE-V1".to_string()),
			mpid_source_code_system: None,
			mpid_source_code_system_version: None,
			mfds_mpid_version: Some("KR-V1".to_string()),
			mfds_mpid: Some("KR-MPID".to_string()),
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			manufacturer_name: None,
			manufacturer_country: None,
			batch_lot_number: None,
			cumulative_dose_first_reaction_value: None,
			cumulative_dose_first_reaction_unit: None,
			gestation_period_exposure_value: None,
			gestation_period_exposure_unit: None,
			action_taken: None,
			fda_additional_info_coded: None,
			fda_additional_info_coded_null_flavor: None,
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_other_characterization: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	fn test_substance(drug_id: Uuid) -> DrugActiveSubstance {
		DrugActiveSubstance {
			id: Uuid::new_v4(),
			drug_id,
			sequence_number: 1,
			deleted: false,
			substance_name: Some("Substance".to_string()),
			substance_termid: Some("BASE-SUB".to_string()),
			substance_termid_version: Some("BASE-SV1".to_string()),
			substance_termid_code_system: Some("TBD-Substance".to_string()),
			mfds_version: Some("KR-SV1".to_string()),
			mfds_id: Some("KR-SUB".to_string()),
			strength_value: None,
			strength_unit: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	fn test_dosage(drug_id: Uuid) -> DosageInformation {
		DosageInformation {
			id: Uuid::new_v4(),
			drug_id,
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_unit: None,
			first_administration_date: None::<Date>,
			first_administration_date_raw: None,
			last_administration_date: None::<Date>,
			last_administration_date_raw: None,
			duration_value: None::<Decimal>,
			duration_unit: None,
			continuing: None,
			batch_lot_number: None,
			batch_lot_number_null_flavor: None,
			dosage_text: Some("row dosage text".to_string()),
			dose_form: None,
			dose_form_null_flavor: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: None,
			route_of_administration_null_flavor: None,
			route_termid: None,
			route_termid_version: None,
			route_termid_code_system: None,
			parent_route: None,
			parent_route_null_flavor: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			parent_route_termid_code_system: None,
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
			deleted: false,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	#[test]
	fn export_g_uses_decimal_number_of_units_for_period_value() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		for unit in ["d", "{cyclical}", "{asnecessary}", "{total}"] {
			let mut dosage = test_dosage(drug_id);
			dosage.number_of_units = Some(Decimal::new(5, 1));
			dosage.frequency_unit = Some(unit.to_string());

			let xml = export_g_drugs_xml(
				&[drug.clone()],
				&[],
				&[dosage],
				&[],
				&[],
				&[],
				&[],
			)
			.expect("export xml");

			assert!(
				xml.contains(&format!("<period value=\"0.5\" unit=\"{unit}\"/>")),
				"{xml}"
			);
			assert!(
				xml.contains("<effectiveTime xsi:type=\"PIVL_TS\"><period"),
				"{xml}"
			);
			assert!(!xml.contains("xsi:type=\"SXPR_TS\""), "{xml}");
			let imported =
				crate::import_sections::g_drug::parse_g_drugs(xml.as_bytes())
					.expect("import exported XML");
			assert_eq!(
				imported[0].dosages[0].number_of_units,
				Some(Decimal::new(5, 1))
			);
			assert_eq!(imported[0].dosages[0].frequency_unit.as_deref(), Some(unit));
		}
	}

	#[test]
	fn export_g_groups_dosage_timing_like_official_examples() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let mut dosage = test_dosage(drug_id);
		dosage.number_of_units = Some(Decimal::new(1, 0));
		dosage.frequency_unit = Some("d".to_string());
		dosage.first_administration_date =
			Some(Date::from_calendar_date(2024, time::Month::January, 1).unwrap());
		dosage.last_administration_date =
			Some(Date::from_calendar_date(2024, time::Month::January, 10).unwrap());
		dosage.duration_value = Some(Decimal::new(9, 0));
		dosage.duration_unit = Some("d".to_string());

		let xml = export_g_drugs_xml(&[drug], &[], &[dosage], &[], &[], &[], &[])
			.expect("export XML");
		let timing = "<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period value=\"1\" unit=\"d\"/></comp><comp xsi:type=\"IVL_TS\" operator=\"A\"><low value=\"20240101\"/><high value=\"20240110\"/></comp><comp xsi:type=\"IVL_TS\" operator=\"A\"><width value=\"9\" unit=\"d\"/></comp></effectiveTime>";
		assert!(xml.contains(timing), "{xml}");
		assert_eq!(xml.matches("xsi:type=\"SXPR_TS\"").count(), 1, "{xml}");

		let imported = crate::import_sections::g_drug::parse_g_drugs(xml.as_bytes())
			.expect("import exported XML");
		let imported = &imported[0].dosages[0];
		assert_eq!(imported.number_of_units, Some(Decimal::new(1, 0)));
		assert_eq!(imported.frequency_unit.as_deref(), Some("d"));
		assert_eq!(
			imported.start_date,
			Some(Date::from_calendar_date(2024, time::Month::January, 1).unwrap())
		);
		assert_eq!(
			imported.end_date,
			Some(Date::from_calendar_date(2024, time::Month::January, 10).unwrap())
		);
		assert_eq!(imported.duration_value, Some(Decimal::new(9, 0)));
		assert_eq!(imported.duration_unit.as_deref(), Some("d"));
	}

	#[test]
	fn export_g_preserves_partial_dosage_date_lexemes() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let mut dosage = test_dosage(drug_id);
		dosage.first_administration_date_raw = Some("200304".to_string());
		dosage.last_administration_date_raw = Some("200305".to_string());

		let xml = export_g_drugs_xml(&[drug], &[], &[dosage], &[], &[], &[], &[])
			.expect("export XML");
		assert!(xml.contains("<low value=\"200304\"/>"), "{xml}");
		assert!(xml.contains("<high value=\"200305\"/>"), "{xml}");
		assert_eq!(partial_ts_lexeme("20240101"), None);
		assert_eq!(partial_ts_lexeme("2024-01"), None);
	}

	#[test]
	fn export_g_blinded_product_matches_official_fda_xsd_path() {
		let mut drug = test_drug(Uuid::new_v4(), Uuid::new_v4());
		drug.investigational_product_blinded = Some(true);
		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.and_then(|path| path.parent())
			.and_then(|path| path.parent())
			.expect("workspace root")
			.to_path_buf();
		let source =
			std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario6.xml"))
				.expect("official FDA example");
		let exported = crate::export::roundtrip::patch_g_drugs_for_authority(
			&source,
			std::slice::from_ref(&drug),
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			RegulatoryAuthority::Fda,
		)
		.expect("patch FDA drug");

		let errors = crate::validation::validate_e2b_xml_xsd(
			exported.as_bytes(),
			&crate::default_xsd_path().expect("official ICH schema"),
		)
		.expect("validate XSD");
		assert!(errors.is_empty(), "{errors:#?}");
		assert!(exported.contains("<code code=\"6\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"blinded\"/><value xsi:type=\"BL\" value=\"true\"/>"));
		let imported =
			crate::import_sections::g_drug::parse_g_drugs(exported.as_bytes())
				.expect("import exported XML");
		assert_eq!(imported[0].investigational_product_blinded, Some(true));

		drug.investigational_product_blinded = Some(false);
		let fragment = write_g_k_drug(
			&drug,
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			RegulatoryAuthority::Fda,
		)
		.expect("export unblinded drug");
		assert!(!fragment.contains("displayName=\"blinded\""), "{fragment}");
	}

	#[test]
	fn export_fda_g_k_2_1_preserves_ndc_code() {
		let drug = test_drug(Uuid::new_v4(), Uuid::new_v4());
		let fragment = write_g_k_drug(
			&drug,
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			RegulatoryAuthority::Fda,
		)
		.expect("export FDA product code");

		assert!(fragment.contains(
			"<code codeSystem=\"2.16.840.1.113883.6.69\" code=\"BASE-MPID\" codeSystemVersion=\"BASE-V1\"/>"
		));
		assert!(!fragment.contains("code=\"MPID\""));
	}

	#[test]
	fn export_fda_g_k_2_1_uses_source_version_when_present() {
		let mut drug = test_drug(Uuid::new_v4(), Uuid::new_v4());
		drug.mpid_source_code_system = Some("2.16.840.1.113883.6.69".to_string());
		drug.mpid_source_code_system_version = Some("201411011202".to_string());
		let fragment = write_g_k_drug(
			&drug,
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			RegulatoryAuthority::Fda,
		)
		.expect("export FDA source MPID");
		assert!(fragment.contains("codeSystemVersion=\"201411011202\""));
	}

	#[test]
	fn export_g_k_7_r_does_not_invent_indication_metadata() {
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, Uuid::new_v4());
		let indication = DrugIndication {
			id: Uuid::new_v4(),
			drug_id,
			sequence_number: 1,
			indication_text: Some("Indication".to_string()),
			indication_text_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		};
		let fragment = write_g_k_drug(
			&drug,
			&[],
			&[],
			&[&indication],
			&[],
			&[],
			&[],
			&[],
			RegulatoryAuthority::Fda,
		)
		.expect("export indication");

		assert!(fragment.contains("<code code=\"19\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"indication\"/>"));
		assert!(fragment.contains("<code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\" displayName=\"sourceReporter\"/>"));
	}

	#[test]
	fn export_g_places_additional_info_after_the_product() {
		let mut drug = test_drug(Uuid::new_v4(), Uuid::new_v4());
		drug.fda_additional_info_coded = Some("1".to_string());
		drug.drug_additional_info_codes_json =
			Some(serde_json::json!([{ "value_code": "2" }]));
		let fragment = write_g_k_drug(
			&drug,
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			RegulatoryAuthority::Fda,
		)
		.expect("export additional information");

		let product_end = fragment.find("</kindOfProduct>").unwrap();
		let additional_info = fragment.match_indices("<code code=\"9\"");
		assert_eq!(additional_info.clone().count(), 2, "{fragment}");
		assert!(additional_info
			.map(|(index, _)| index)
			.all(|index| index > product_end));
	}

	#[test]
	fn export_g_normalizes_stored_dose_scale() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let mut dosage = test_dosage(drug_id);
		dosage.dose_value = Some(Decimal::new(10_000_000, 5));

		let xml = export_g_drugs_xml(&[drug], &[], &[dosage], &[], &[], &[], &[])
			.expect("export xml");

		assert!(xml.contains("<doseQuantity value=\"100\"/>"), "{xml}");
		assert!(!xml.contains("value=\"100.00000\""), "{xml}");
	}

	#[test]
	fn export_g_normalizes_time_units_and_decimals() {
		let mut drug = test_drug(Uuid::new_v4(), Uuid::new_v4());
		drug.drug_authorization_number = Some("FDA-IND-123456".to_string());
		drug.manufacturer_country = Some("US".to_string());
		drug.gestation_period_exposure_value = Some(Decimal::new(2000, 2));
		drug.gestation_period_exposure_unit = Some("804".to_string());
		let mut dosage = test_dosage(drug.id);
		dosage.duration_value = Some(Decimal::new(200, 2));
		dosage.duration_unit = Some("day".to_string());

		let xml = export_g_drugs_xml(&[drug], &[], &[dosage], &[], &[], &[], &[])
			.expect("export XML");

		assert!(xml.contains("<author typeCode=\"AUT\"><territorialAuthority classCode=\"TERR\"><territory classCode=\"NAT\" determinerCode=\"INSTANCE\"><code code=\"US\" codeSystem=\"1.0.3166.1.2.2\"/>"), "{xml}");
		assert!(xml.contains("displayName=\"gestationPeriod\"/><value xsi:type=\"PQ\" value=\"20\" unit=\"d\"/>"), "{xml}");
		assert!(xml.contains("<width value=\"2\" unit=\"d\"/>"), "{xml}");
	}

	#[test]
	fn export_g_emits_dosage_text_only_inside_repeated_dosage() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let dosage = test_dosage(drug_id);

		let xml = export_g_drugs_xml(&[drug], &[], &[dosage], &[], &[], &[], &[])
			.expect("export xml");
		let parser = libxml::parser::Parser::default();
		let doc = parser.parse_string(&xml).expect("parse exported xml");
		let mut xpath = libxml::xpath::Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		let drug_text = xpath
			.findvalue(
				"//hl7:organizer/hl7:component/hl7:substanceAdministration/hl7:text",
				None,
			)
			.unwrap();
		let dosage_text = xpath
			.findvalue(
				"//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:text",
				None,
			)
			.unwrap();

		assert_eq!(drug_text, "");
		assert_eq!(dosage_text, "row dosage text");
	}

	#[test]
	fn export_g_does_not_alias_mfds_fields_to_base_paths() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let substance = test_substance(drug_id);

		let xml = export_g_drugs_xml(&[drug], &[substance], &[], &[], &[], &[], &[])
			.expect("export xml");
		let parser = libxml::parser::Parser::default();
		let doc = parser.parse_string(&xml).expect("parse exported xml");
		let mut xpath = libxml::xpath::Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		let mpid = xpath
			.findvalue(
				"//hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension",
				None,
			)
			.unwrap();
		assert_eq!(mpid, "BASE-MPID");
		let mpid_version = xpath
			.findvalue(
				"//hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion",
				None,
			)
			.unwrap();
		assert_eq!(mpid_version, "BASE-V1");
		let substance_code = xpath
			.findvalue(
				"//hl7:ingredient/hl7:ingredientSubstance/hl7:code/@code",
				None,
			)
			.unwrap();
		assert_eq!(substance_code, "BASE-SUB");
		let substance_version = xpath
			.findvalue(
				"//hl7:ingredient/hl7:ingredientSubstance/hl7:code/@codeSystemVersion",
				None,
			)
			.unwrap();
		assert_eq!(substance_version, "BASE-SV1");
		let substance_code_system = xpath
			.findvalue(
				"//hl7:ingredient/hl7:ingredientSubstance/hl7:code/@codeSystem",
				None,
			)
			.unwrap();
		assert_eq!(substance_code_system, "TBD-Substance");

		assert!(
			!xml.contains("KR-MPID") && !xml.contains("KR-V1"),
			"MFDS MPID values must wait for verified MFDS XML paths"
		);
		assert!(
			!xml.contains("KR-SUB") && !xml.contains("KR-SV1"),
			"MFDS substance values must wait for verified MFDS XML paths"
		);
	}

	#[test]
	fn exports_dosage_companions_as_xml_null_flavor_attributes() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let mut dosage = test_dosage(drug_id);
		dosage.route_of_administration_null_flavor = Some("ASKU".to_string());
		dosage.dose_form_null_flavor = Some("UNK".to_string());
		dosage.parent_route_null_flavor = Some("NASK".to_string());
		dosage.batch_lot_number_null_flavor = Some("UNK".to_string());

		let xml = export_g_drugs_xml(&[drug], &[], &[dosage], &[], &[], &[], &[])
			.expect("export xml");
		assert!(xml.contains("<routeCode nullFlavor=\"ASKU\"/>"));
		assert!(xml.contains("<formCode nullFlavor=\"UNK\">"));
		assert!(xml.contains("<value xsi:type=\"CE\" nullFlavor=\"NASK\">"));
		assert!(xml.contains(
			"<productInstanceInstance><id nullFlavor=\"NI\"/><lotNumberText nullFlavor=\"UNK\"/></productInstanceInstance>"
		));
	}

	#[test]
	fn exports_dosage_route_code_systems() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let mut dosage = test_dosage(drug_id);
		dosage.route_termid = Some("001".to_string());
		dosage.route_termid_code_system = Some("0.4.0.127.0.16.1.1.2.6".to_string());
		dosage.parent_route_termid = Some("002".to_string());
		dosage.parent_route_termid_code_system =
			Some("2.16.840.1.113883.3.989.2.1.1.14".to_string());

		let xml = export_g_drugs_xml(&[drug], &[], &[dosage], &[], &[], &[], &[])
			.expect("export route code systems");
		assert!(xml.contains(
			"<routeCode code=\"001\" codeSystem=\"0.4.0.127.0.16.1.1.2.6\""
		));
		assert!(xml.contains("<value xsi:type=\"CE\" code=\"002\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.14\""));
	}

	#[test]
	fn export_g_uses_mfds_product_and_substance_codes_for_mfds_authority() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let substance = test_substance(drug_id);
		let xml = write_g_k_drug(
			&drug,
			&[&substance],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			RegulatoryAuthority::Mfds,
		)
		.expect("export MFDS drug");

		assert!(xml.contains(
			"<code codeSystem=\"2.16.840.1.113883.3.989.5.1.10.2.1\" code=\"KR-MPID\" codeSystemVersion=\"KR-V1\"/>"
		));
		assert!(
			xml.find("code=\"KR-MPID\"").unwrap()
				< xml.find("<name>Drug A</name>").unwrap(),
			"MFDS product code must precede name per the E2B(R3) XSD"
		);
		assert!(xml.contains(
			"<code codeSystem=\"2.16.840.1.113883.3.989.5.1.10.2.2\" code=\"KR-SUB\" codeSystemVersion=\"KR-SV1\"/>"
		));
	}

	#[test]
	fn export_fda_devices_preserves_both_repeat_levels() {
		let drug = test_drug(Uuid::new_v4(), Uuid::new_v4());
		let make_device = |sequence_number, brand: &str| FdaDeviceInformation {
			id: Uuid::new_v4(),
			drug_id: drug.id,
			sequence_number,
			malfunction: Some(true),
			device_brand_name: Some(brand.to_string()),
			device_brand_name_null_flavor: None,
			common_device_name: Some("Syringe".to_string()),
			common_device_name_null_flavor: None,
			device_product_code: Some("FMF".to_string()),
			manufacturer_name: Some("Device Maker".to_string()),
			manufacturer_address: Some("1 Device Way".to_string()),
			manufacturer_city: Some("Silver Spring".to_string()),
			manufacturer_state: Some("MD".to_string()),
			manufacturer_country: Some("US".to_string()),
			device_usage: Some("2".to_string()),
			device_lot_number: Some("LOT-DEVICE".to_string()),
			operator_of_device: Some("1".to_string()),
			deleted: false,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		};
		let first = make_device(1, "Device A");
		let second = make_device(2, "Device B");
		let codes = [
			FdaDeviceCode {
				id: Uuid::new_v4(),
				device_id: first.id,
				element: "device_problem".to_string(),
				sequence_number: 1,
				value_code: "4001".to_string(),
				deleted: false,
				created_at: OffsetDateTime::now_utc(),
				updated_at: OffsetDateTime::now_utc(),
				created_by: Uuid::new_v4(),
				updated_by: None,
			},
			FdaDeviceCode {
				id: Uuid::new_v4(),
				device_id: first.id,
				element: "device_problem".to_string(),
				sequence_number: 2,
				value_code: "3003".to_string(),
				deleted: false,
				created_at: OffsetDateTime::now_utc(),
				updated_at: OffsetDateTime::now_utc(),
				created_by: Uuid::new_v4(),
				updated_by: None,
			},
			FdaDeviceCode {
				id: Uuid::new_v4(),
				device_id: first.id,
				element: "follow_up_type".to_string(),
				sequence_number: 1,
				value_code: "2".to_string(),
				deleted: false,
				created_at: OffsetDateTime::now_utc(),
				updated_at: OffsetDateTime::now_utc(),
				created_by: Uuid::new_v4(),
				updated_by: None,
			},
			FdaDeviceCode {
				id: Uuid::new_v4(),
				device_id: first.id,
				element: "remedial_action".to_string(),
				sequence_number: 1,
				value_code: "6".to_string(),
				deleted: false,
				created_at: OffsetDateTime::now_utc(),
				updated_at: OffsetDateTime::now_utc(),
				created_by: Uuid::new_v4(),
				updated_by: None,
			},
		];
		let xml = write_g_k_drug(
			&drug,
			&[],
			&[],
			&[],
			&[],
			&[&first, &second],
			&codes.iter().collect::<Vec<_>>(),
			&[],
			RegulatoryAuthority::Fda,
		)
		.expect("export FDA devices");

		assert_eq!(xml.matches("<part classCode=\"PART\">").count(), 2);
		assert_eq!(xml.matches("code=\"C54451\"").count(), 2);
		assert!(xml.contains("<name>Device A</name>"));
		assert!(xml.contains("<name>Device B</name>"));
		assert!(xml.contains("code=\"C54592\""));
		assert!(xml.contains("code=\"C54594\""));
		assert!(xml.contains("code=\"C54595\""));
		assert!(xml.contains(
			"codeSystem=\"2.16.840.1.113883.3.989.5.1.2.1.1.6\" code=\"1\""
		));
		assert!(xml.contains("<name>Device Maker</name>"));
		assert!(xml.contains("<streetAddressLine>1 Device Way</streetAddressLine>"));
		assert!(xml.contains("<city>Silver Spring</city>"));
		assert!(xml.contains("<state>MD</state>"));
		assert!(xml.contains("<country>US</country>"));
		assert!(xml.contains("<lotNumberText>LOT-DEVICE</lotNumberText>"));

		let ich = write_g_k_drug(
			&drug,
			&[],
			&[],
			&[],
			&[],
			&[&first, &second],
			&codes.iter().collect::<Vec<_>>(),
			&[],
			RegulatoryAuthority::Ich,
		)
		.expect("export ICH without FDA devices");
		assert!(!ich.contains("<partProduct classCode=\"DEV\""));
	}

	#[test]
	fn export_g_emits_fda_other_drug_role_only_for_fda_authority() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let mut drug = test_drug(drug_id, case_id);
		drug.fda_other_characterization = Some("1".to_string());

		let fda = write_g_k_causality_assessments(
			&drug,
			&[],
			&[],
			RegulatoryAuthority::Fda,
		)
		.expect("FDA causality");
		let mfds = write_g_k_causality_assessments(
			&drug,
			&[],
			&[],
			RegulatoryAuthority::Mfds,
		)
		.expect("MFDS causality");

		assert!(fda.contains("2.16.840.1.113883.3.989.5.1.2.1.1.8"));
		assert!(fda.contains("code=\"1\" displayName=\"Similar Device\""));
		assert!(!mfds.contains("2.16.840.1.113883.3.989.5.1.2.1.1.8"));
	}

	#[test]
	fn preserves_invalid_g_k_1_for_non_blocking_export() {
		let mut drug = test_drug(Uuid::new_v4(), Uuid::new_v4());
		drug.drug_characterization.clear();
		assert!(write_g_k_1_causality(&drug).is_empty());

		drug.drug_characterization = "9".to_string();
		assert!(write_g_k_1_causality(&drug).contains("code=\"9\""));
	}

	#[test]
	fn mfds_relatedness_uses_kr1_code_systems() {
		let assessment = test_assessment();
		let relatedness = RelatednessAssessment {
			id: Uuid::new_v4(),
			drug_reaction_assessment_id: assessment.id,
			sequence_number: 1,
			source_of_assessment: None,
			method_of_assessment: Some("free text".to_string()),
			method_of_assessment_kr1: Some("1".to_string()),
			result_of_assessment: Some("free text".to_string()),
			result_of_assessment_kr1: Some("3".to_string()),
			result_of_assessment_kr1_null_flavor: None,
			result_of_assessment_kr2: None,
			deleted: false,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		};
		let xml = write_g_k_9_causality(
			Uuid::new_v4(),
			&assessment,
			&relatedness,
			RegulatoryAuthority::Mfds,
		);

		assert!(xml.contains("methodCode codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.4\" code=\"1\""));
		assert!(xml.contains(
			"codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.5\" code=\"3\""
		));
		assert!(!xml.contains("free text"));

		let mut null_flavor = relatedness;
		null_flavor.result_of_assessment_kr1 = None;
		null_flavor.result_of_assessment_kr1_null_flavor = Some("NA".to_string());
		let xml = write_g_k_9_causality(
			Uuid::new_v4(),
			&assessment,
			&null_flavor,
			RegulatoryAuthority::Mfds,
		);
		assert!(xml.contains("<value xsi:type=\"CE\" nullFlavor=\"NA\"/>"));
		assert!(!xml.contains("code=\"NA\""));
	}

	fn test_assessment() -> DrugReactionAssessment {
		DrugReactionAssessment {
			id: Uuid::new_v4(),
			drug_id: Uuid::new_v4(),
			reaction_id: Uuid::new_v4(),
			administration_start_interval_value: None,
			administration_start_interval_unit: None,
			last_dose_interval_value: None,
			last_dose_interval_unit: None,
			recurrence_action: Some("1".to_string()),
			reaction_recurred: Some("1".to_string()),
			dechallenge_result: None,
			expectedness: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	#[test]
	fn recurrence_export_emits_only_standard_code31_no_invented_codes() {
		let xml = write_g_k_9_i_recurrence(&test_assessment());

		// The recurrence answer is carried by the standard G.k.9.i.4 observation (code 31).
		assert!(
			xml.contains("code=\"31\""),
			"expected the standard recurrence observation (code 31)"
		);
		// G.k.8.r.1 / G.k.8.r.2 are not real ICH/FDA/MFDS codes and must not be emitted.
		assert!(
			!xml.contains("G.k.8.r"),
			"non-standard recurrence codes G.k.8.r.* must not be exported: {xml}"
		);
	}

	#[test]
	fn export_g_groups_time_intervals_before_recurrence_observations() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let mut first = test_assessment();
		first.drug_id = drug_id;
		first.administration_start_interval_value = Some(Decimal::new(3, 0));
		first.administration_start_interval_unit = Some("d".to_string());
		let mut second = test_assessment();
		second.drug_id = drug_id;
		second.administration_start_interval_value = Some(Decimal::new(44, 0));
		second.administration_start_interval_unit = Some("d".to_string());
		second.last_dose_interval_value = Some(Decimal::new(28, 0));
		second.last_dose_interval_unit = Some("d".to_string());

		let Ok(source) = std::fs::read(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/../../../docs/exporter/mfds/1-1_ExampleCase_literature_KR_initial_v1_0_샘플.xml"
		)) else {
			return;
		};
		let xml = crate::export::roundtrip::patch_g_drugs_for_authority(
			&source,
			&[drug],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[first, second],
			&[],
			RegulatoryAuthority::Mfds,
		)
		.expect("patch MFDS drug");

		let parser = libxml::parser::Parser::default();
		let doc = parser.parse_string(&xml).expect("parse export");
		let mut xpath = libxml::xpath::Context::new(&doc).expect("create XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("register namespace");
		let node = xpath
			.findnodes(
				&format!("//hl7:substanceAdministration[hl7:id[@root='{drug_id}']]"),
				None,
			)
			.expect("find drug")
			.into_iter()
			.next()
			.expect("drug node");
		let mut seen_relationship2 = false;
		let mut relationship1_count = 0;
		for child in node.get_child_nodes() {
			match child.get_name().as_str() {
				"outboundRelationship2" => seen_relationship2 = true,
				"outboundRelationship1" => {
					assert!(
						!seen_relationship2,
						"outboundRelationship1 emitted after outboundRelationship2"
					);
					relationship1_count += 1;
				}
				_ => {}
			}
		}
		assert_eq!(relationship1_count, 3);

		let errors = crate::validation::validate_e2b_xml_xsd(
			xml.as_bytes(),
			&crate::default_xsd_path().expect("official ICH schema"),
		)
		.expect("validate XSD");
		assert!(errors.is_empty(), "{errors:#?}");
	}
}
