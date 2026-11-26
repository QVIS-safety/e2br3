use crate::common::{date, decimal, fixture};
use xml::import_sections::g_drug::parse_g_drugs;

fn g_fixture(name: &str) -> Vec<u8> {
	String::from_utf8(fixture(name))
		.expect("UTF-8 fixture")
		.replace("201411011202", "2014110112")
		.replace("201401011041", "2014010110")
		.into_bytes()
}

#[test]
fn import_g_section_all_fields_from_scenario6() {
	let xml = g_fixture("FAERS2022Scenario6.xml");

	let drugs = parse_g_drugs(&xml).expect("parse");
	assert_eq!(drugs.len(), 2);

	let first = &drugs[0];
	assert_eq!(
		first.xml_id.as_deref(),
		Some("3c91b4d5-e039-4a7a-9c30-67671b0ef9e4")
	);
	assert_eq!(first.sequence_number, 1);
	assert_eq!(first.medicinal_product, "Drug A");
	assert_eq!(first.drug_characterization, "1");
	assert_eq!(first.mpid.as_deref(), Some("894444-28525-765"));
	assert_eq!(
		first.mpid_source_code_system_version.as_deref(),
		Some("2014110112")
	);
	assert_eq!(first.phpid, None);
	assert_eq!(first.phpid_version, None);
	assert_eq!(first.investigational_product_blinded, Some(true));
	assert_eq!(first.obtain_drug_country.as_deref(), Some("US"));
	assert_eq!(
		first.drug_authorization_number.as_deref(),
		Some("BLA012345")
	);
	// FDA.G.k.2.2.1 is intentionally unsupported until a verified source
	// mapping or fixture is available, so the parser currently exposes no
	// separate generic-name value.
	assert_eq!(
		first.manufacturer_name.as_deref(),
		Some("Pharmacia and Upjohn Company")
	);
	assert_eq!(first.manufacturer_country.as_deref(), Some("US"));
	assert_eq!(first.batch_lot_number, None);
	assert_eq!(
		first.cumulative_dose_first_reaction_value,
		Some(decimal("150"))
	);
	assert_eq!(
		first.cumulative_dose_first_reaction_unit.as_deref(),
		Some("mg")
	);
	assert_eq!(first.gestation_period_exposure_value, Some(decimal("10")));
	assert_eq!(first.gestation_period_exposure_unit.as_deref(), Some("wk"));
	assert_eq!(first.action_taken.as_deref(), Some("2"));
	// The fixture's ICH codes must not be mistaken for the separate FDA code.
	assert_eq!(first.fda_additional_info_coded.as_deref(), Some("1"));
	assert_eq!(
		first.drug_additional_info_codes_json,
		Some(serde_json::json!(["4", "9", "10"]))
	);
	assert_eq!(
		first.fda_specialized_product_category.as_deref(),
		Some("C102835")
	);
	let device = &first.devices[0];
	assert_eq!(device.device_brand_name.as_deref(), Some("Brand Name"));
	assert_eq!(
		device.common_device_name.as_deref(),
		Some("Common Device Name")
	);
	assert_eq!(device.device_product_code.as_deref(), Some("FMF"));
	assert_eq!(
		device.manufacturer_name.as_deref(),
		Some("Manufacturer Name")
	);
	assert_eq!(
		device.manufacturer_address.as_deref(),
		Some("Manufacturer Address 123 google home drive")
	);
	assert_eq!(
		device.manufacturer_city.as_deref(),
		Some("Manufacturer City")
	);
	assert_eq!(
		device.manufacturer_state.as_deref(),
		Some("Manufacturer State")
	);
	assert_eq!(device.manufacturer_country.as_deref(), Some("US"));
	assert_eq!(
		device.device_lot_number.as_deref(),
		Some("4577BN2-product-2-device1")
	);
	assert_eq!(device.operator_of_device.as_deref(), Some("1"));

	assert_eq!(first.substances.len(), 2);
	let first_substance = &first.substances[0];
	assert_eq!(
		first_substance.substance_name.as_deref(),
		Some("TERBINAFINE HYDROCHLORIDE")
	);
	assert_eq!(
		first_substance.substance_termid.as_deref(),
		Some("012C11ZU6G")
	);
	assert_eq!(
		first_substance.substance_termid_version.as_deref(),
		Some("12JUL2013")
	);
	assert_eq!(first_substance.strength_value, Some(decimal("10")));
	assert_eq!(first_substance.strength_unit.as_deref(), Some("mg"));

	assert_eq!(first.dosages.len(), 2);
	let first_dosage = &first.dosages[0];
	assert_eq!(
		first_dosage.dosage_text.as_deref(),
		Some("unstructured dosing information, e.g. take by mouth 2 times a day with food ")
	);
	assert_eq!(first_dosage.number_of_units, Some(decimal("10")));
	assert_eq!(first_dosage.frequency_unit.as_deref(), Some("d"));
	assert_eq!(first_dosage.start_date, Some(date(2009, 1, 1)));
	assert_eq!(first_dosage.start_date_null_flavor, None);
	assert_eq!(first_dosage.end_date, Some(date(2009, 1, 1)));
	assert_eq!(first_dosage.end_date_null_flavor, None);
	assert_eq!(first_dosage.duration_value, Some(decimal("4")));
	assert_eq!(first_dosage.duration_unit.as_deref(), Some("wk"));
	assert_eq!(first_dosage.dose_value, Some(decimal("10")));
	assert_eq!(first_dosage.dose_unit.as_deref(), Some("10.a"));
	assert_eq!(
		first_dosage.route.as_deref(),
		Some("Free text route content")
	);
	assert_eq!(
		first_dosage.route_termid_version.as_deref(),
		Some("2014.10.30")
	);
	assert_eq!(
		first_dosage.dose_form.as_deref(),
		Some("Big, round and colored white")
	);
	assert_eq!(first_dosage.dose_form_termid.as_deref(), Some("C42998"));
	assert_eq!(
		first_dosage.dose_form_termid_version.as_deref(),
		Some("2014.10.30")
	);
	assert_eq!(first_dosage.batch_lot.as_deref(), Some("4577BN2"));
	assert_eq!(first_dosage.parent_route_termid, None);
	assert_eq!(first_dosage.parent_route_termid_version, None);
	assert_eq!(first_dosage.parent_route, None);

	assert_eq!(first.indications.len(), 2);
	let first_indication = &first.indications[0];
	assert_eq!(
		first_indication.text.as_deref(),
		Some("Medication was prescribed to address sickness to the stomach.")
	);
	assert_eq!(first_indication.version.as_deref(), Some("12.0"));
	assert_eq!(first_indication.code.as_deref(), Some("10028813"));

	assert!(
		first.characteristics.is_empty(),
		"FDA category has a dedicated field"
	);
}

#[test]
fn import_g_dosage_null_flavor_from_scenario1() {
	let xml = g_fixture("FAERS2022Scenario1.xml");
	let drugs = parse_g_drugs(&xml).expect("parse");
	let first = &drugs[0];
	let first_dosage = &first.dosages[0];

	assert_eq!(
		first.drug_authorization_number.as_deref(),
		Some("NDA012345")
	);
	assert_eq!(first_dosage.number_of_units, None);
	assert_eq!(first_dosage.start_date, None);
	assert_eq!(first_dosage.start_date_null_flavor.as_deref(), Some("ASKU"));
	assert_eq!(first_dosage.end_date, None);
	assert_eq!(first_dosage.end_date_null_flavor.as_deref(), Some("ASKU"));
}

#[test]
fn import_g_ignores_retired_mfds_device_characteristics() {
	let source = String::from_utf8(g_fixture("FAERS2022Scenario6.xml"))
		.expect("UTF-8")
		.replace("C94031", "OTHER_CHARACTERISTIC")
		.replace("FDA Specialized Product Category", "Other characteristic");
	let original = parse_g_drugs(source.as_bytes()).unwrap();
	let replaced = source.replace("OTHER_CHARACTERISTIC", "KR_DVC_SN");
	assert_ne!(source, replaced);
	let drugs = parse_g_drugs(replaced.as_bytes()).unwrap();
	assert_eq!(
		drugs[0].characteristics.len(),
		original[0].characteristics.len() - 1
	);
	assert!(drugs
		.iter()
		.flat_map(|drug| &drug.characteristics)
		.all(|value| value.code.as_deref() != Some("KR_DVC_SN")));
}

#[test]
fn import_g_category_aliases_do_not_create_duplicate_characteristic_rows() {
	let source =
		String::from_utf8(g_fixture("FAERS2022Scenario6.xml")).expect("UTF-8");
	let code_start = source.find("<code code=\"C94031\"").expect("category code");
	let start = source[..code_start]
		.rfind("<subjectOf")
		.expect("category owner");
	let end = code_start
		+ source[code_start..]
			.find("</subjectOf>")
			.expect("category end")
		+ "</subjectOf>".len();
	let category = &source[start..end];
	let owner_end = end
		+ source[end..]
			.find("</asManufacturedProduct>")
			.expect("product end")
		+ "</asManufacturedProduct>".len();
	let nested = format!("{}{}<part><partProduct><asManufacturedProduct>{category}</asManufacturedProduct></partProduct></part>{}", &source[..start], &source[end..owner_end], &source[owner_end..]);
	for placement in [&source, &nested] {
		for code in ["C94031", "FDAGK101", "FDA.G.k.10.1"] {
			let xml = placement
				.replace("C94031", code)
				.replace("displayName=\"FDA Specialized Product Category\"", "");
			let drugs = parse_g_drugs(xml.as_bytes()).expect("category import");
			assert_eq!(
				drugs[0].fda_specialized_product_category.as_deref(),
				Some("C102835"),
				"{code}"
			);
			assert!(drugs[0].characteristics.is_empty(), "{code} duplicated");
		}
	}
	let unknown = source
		.replace("C94031", "OTHER_CHARACTERISTIC")
		.replace("FDA Specialized Product Category", "Other characteristic");
	let drugs = parse_g_drugs(unknown.as_bytes()).expect("unknown characteristic");
	assert_eq!(drugs[0].characteristics.len(), 1);
	assert_eq!(
		drugs[0].characteristics[0].value_code.as_deref(),
		Some("C102835")
	);
	assert_eq!(drugs[0].fda_specialized_product_category, None);
}

#[test]
fn import_g_keeps_device_lot_out_of_local_drug_batch_lot() {
	let drugs = parse_g_drugs(&g_fixture("FAERS2022Scenario6.xml")).expect("parse");
	assert_eq!(drugs[0].batch_lot_number, None);
	assert_eq!(
		drugs[0].devices[0].device_lot_number.as_deref(),
		Some("4577BN2-product-2-device1")
	);
	assert!(drugs[0]
		.characteristics
		.iter()
		.all(|value| value.code.as_deref() != Some("C54451")));
}

#[test]
fn import_g_ignores_legacy_local_lot_paths() {
	let source = String::from_utf8(g_fixture("FAERS2022Scenario6.xml"))
		.expect("UTF-8 fixture");
	let baseline = parse_g_drugs(source.as_bytes()).expect("baseline");
	let legacy = source.replace(
		"</kindOfProduct>",
		"<part><partProduct><instanceOfKind><productInstanceInstance><lotNumberText>LOCAL-PART</lotNumberText></productInstanceInstance></instanceOfKind></partProduct></part></kindOfProduct><productInstanceInstance><lotNumberText>LOCAL-DIRECT</lotNumberText></productInstanceInstance>",
	);
	assert_ne!(legacy, source);
	let imported = parse_g_drugs(legacy.as_bytes()).expect("legacy XML");
	assert_eq!(imported.len(), baseline.len());
	for (actual, expected) in imported.iter().zip(&baseline) {
		assert_eq!(actual.batch_lot_number, None);
		assert_eq!(actual.dosages.len(), expected.dosages.len());
		for (actual, expected) in actual.dosages.iter().zip(&expected.dosages) {
			assert_eq!(actual.batch_lot, expected.batch_lot);
		}
		assert_eq!(actual.devices.len(), expected.devices.len());
		for (actual, expected) in actual.devices.iter().zip(&expected.devices) {
			assert_eq!(actual.device_lot_number, expected.device_lot_number);
		}
	}
}
