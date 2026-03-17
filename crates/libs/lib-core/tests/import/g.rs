use crate::common::{date, decimal, fixture};
use lib_core::xml::import_sections::g_drug::parse_g_drugs;
use sqlx::types::Uuid;

#[test]
fn import_g_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let drugs = parse_g_drugs(&xml).expect("parse");
	assert_eq!(drugs.len(), 2);

	let first = &drugs[0];
	assert_eq!(
		first.xml_id,
		Some(
			Uuid::parse_str("3c91b4d5-e039-4a7a-9c30-67671b0ef9e4")
				.expect("valid uuid"),
		)
	);
	assert_eq!(first.sequence_number, 1);
	assert_eq!(first.medicinal_product, "Drug A");
	assert_eq!(first.brand_name, None);
	assert_eq!(first.drug_characterization, "1");
	assert_eq!(first.mpid.as_deref(), Some("894444-28525-765"));
	assert_eq!(first.mpid_version.as_deref(), Some("2014110112"));
	assert_eq!(first.phpid, None);
	assert_eq!(first.phpid_version, None);
	assert_eq!(first.investigational_product_blinded, None);
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
	assert_eq!(first.dosage_text, None);
	assert_eq!(first.action_taken.as_deref(), Some("2"));
	assert_eq!(first.rechallenge.as_deref(), Some("2"));
	assert_eq!(first.parent_route, None);
	assert_eq!(first.parent_route_termid, None);
	assert_eq!(first.parent_route_termid_version, None);
	assert_eq!(
		first.parent_dosage_text.as_deref(),
		Some("Somthing seemed strange about the way it went down.")
	);
	assert_eq!(first.fda_additional_info_coded.as_deref(), Some("4"));
	assert_eq!(
		first.fda_specialized_product_category.as_deref(),
		Some("C102835")
	);
	assert_eq!(first.fda_device_brand_name.as_deref(), Some("Brand Name"));
	assert_eq!(
		first.fda_common_device_name.as_deref(),
		Some("Common Device Name")
	);
	assert_eq!(first.fda_device_product_code.as_deref(), Some("FMF"));
	assert_eq!(
		first.fda_device_manufacturer_name.as_deref(),
		Some("Manufacturer Name")
	);
	assert_eq!(
		first.fda_device_manufacturer_address.as_deref(),
		Some("Manufacturer Address 123 google home drive")
	);
	assert_eq!(
		first.fda_device_manufacturer_city.as_deref(),
		Some("Manufacturer City")
	);
	assert_eq!(
		first.fda_device_manufacturer_state.as_deref(),
		Some("Manufacturer State")
	);
	assert_eq!(first.fda_device_manufacturer_country.as_deref(), Some("US"));
	assert_eq!(
		first.fda_device_lot_number.as_deref(),
		Some("4577BN2-product-2-device1")
	);
	assert_eq!(first.fda_operator_of_device.as_deref(), Some("1"));

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
	assert_eq!(first_dosage.number_of_units, Some(10));
	assert_eq!(first_dosage.frequency_value, Some(decimal("10")));
	assert_eq!(first_dosage.frequency_unit.as_deref(), Some("d"));
	assert_eq!(first_dosage.start_date, Some(date(2009, 1, 1)));
	assert_eq!(first_dosage.start_time, None);
	assert_eq!(first_dosage.start_date_null_flavor, None);
	assert_eq!(first_dosage.end_date, Some(date(2009, 1, 1)));
	assert_eq!(first_dosage.end_time, None);
	assert_eq!(first_dosage.end_date_null_flavor, None);
	assert_eq!(first_dosage.duration_value, Some(decimal("4")));
	assert_eq!(first_dosage.duration_unit.as_deref(), Some("wk"));
	assert_eq!(first_dosage.dose_value, Some(decimal("10")));
	assert_eq!(first_dosage.dose_unit.as_deref(), Some("10.a"));
	assert_eq!(first_dosage.route, None);
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

	assert_eq!(first.characteristics.len(), 10);
	let first_characteristic = &first.characteristics[0];
	assert_eq!(first_characteristic.code.as_deref(), Some("C94031"));
	assert_eq!(
		first_characteristic.code_system.as_deref(),
		Some("2.16.840.1.113883.3.26.1.1")
	);
	assert_eq!(
		first_characteristic.code_display_name.as_deref(),
		Some("FDA Specialized Product Category")
	);
	assert_eq!(first_characteristic.value_type.as_deref(), Some("CE"));
	assert_eq!(first_characteristic.value_value, None);
	assert_eq!(first_characteristic.value_code.as_deref(), Some("C102835"));
	assert_eq!(
		first_characteristic.value_code_system.as_deref(),
		Some("2.16.840.1.113883.3.26.1.1")
	);
	assert_eq!(
		first_characteristic.value_display_name.as_deref(),
		Some("Type 2: Prefilled Drug Delivery Device/System (syringe, patch, etc.)")
	);

	let last_characteristic = &first.characteristics[9];
	assert_eq!(last_characteristic.code.as_deref(), Some("1"));
	assert_eq!(
		last_characteristic.code_system.as_deref(),
		Some("2.16.840.1.113883.3.989.5.1.2.1.1.6")
	);
	assert_eq!(
		last_characteristic.code_display_name.as_deref(),
		Some("Health Professional")
	);
	assert_eq!(last_characteristic.value_type, None);
	assert_eq!(last_characteristic.value_value, None);
	assert_eq!(last_characteristic.value_code, None);
	assert_eq!(last_characteristic.value_code_system, None);
	assert_eq!(last_characteristic.value_display_name, None);
}

#[test]
fn import_g_dosage_null_flavor_from_scenario1() {
	let xml = fixture("FAERS2022Scenario1.xml");
	let drugs = parse_g_drugs(&xml).expect("parse");
	let first = &drugs[0];
	let first_dosage = &first.dosages[0];

	assert_eq!(
		first.drug_authorization_number.as_deref(),
		Some("NDA012345")
	);
	assert_eq!(first_dosage.number_of_units, None);
	assert_eq!(first_dosage.start_date, None);
	assert_eq!(first_dosage.start_time, None);
	assert_eq!(first_dosage.start_date_null_flavor.as_deref(), Some("ASKU"));
	assert_eq!(first_dosage.end_date, None);
	assert_eq!(first_dosage.end_time, None);
	assert_eq!(first_dosage.end_date_null_flavor.as_deref(), Some("ASKU"));
}
