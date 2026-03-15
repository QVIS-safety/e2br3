use lib_core::model::drug::{
	parse_fda_device_info_json, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use lib_core::model::drug_recurrence::DrugRecurrenceInformation;
use serial_test::serial;

use crate::common::{decimal, import_fixture, list_by_uuid};

#[serial]
#[tokio::test]
async fn imports_g_drug_models() {
	let imported = import_fixture("FAERS2022Scenario6.xml").await;
	let drugs: Vec<DrugInformation> = list_by_uuid(
		&imported,
		"SELECT * FROM drug_information WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;

	assert_eq!(drugs.len(), 2);

	assert_drug_one(&imported, &drugs[0]);
	assert_drug_two(&imported, &drugs[1]);

	let substances_one: Vec<DrugActiveSubstance> = list_by_uuid(
		&imported,
		"SELECT * FROM drug_active_substances WHERE drug_id = $1 ORDER BY sequence_number",
		drugs[0].id,
	)
	.await;
	let indications_one: Vec<DrugIndication> = list_by_uuid(
		&imported,
		"SELECT * FROM drug_indications WHERE drug_id = $1 ORDER BY sequence_number",
		drugs[0].id,
	)
	.await;
	let characteristics_one: Vec<DrugDeviceCharacteristic> = list_by_uuid(
		&imported,
		"SELECT * FROM drug_device_characteristics WHERE drug_id = $1 ORDER BY sequence_number",
		drugs[0].id,
	)
	.await;

	assert_eq!(substances_one.len(), 2);
	assert_eq!(substances_one[0].drug_id, drugs[0].id);
	assert_eq!(substances_one[0].sequence_number, 1);
	assert_eq!(
		substances_one[0].substance_name.as_deref(),
		Some("TERBINAFINE HYDROCHLORIDE")
	);
	assert_eq!(
		substances_one[0].substance_termid.as_deref(),
		Some("012C11ZU6G")
	);
	assert_eq!(
		substances_one[0].substance_termid_version.as_deref(),
		Some("12JUL2013")
	);
	assert_eq!(substances_one[0].strength_value, Some(decimal("10")));
	assert_eq!(substances_one[0].strength_unit.as_deref(), Some("mg"));

	assert_eq!(substances_one[1].drug_id, drugs[0].id);
	assert_eq!(substances_one[1].sequence_number, 2);
	assert_eq!(
		substances_one[1].substance_name.as_deref(),
		Some("ARGEMONE MEXICANA SEED OIL")
	);
	assert_eq!(
		substances_one[1].substance_termid.as_deref(),
		Some("0Q5GP4Y4FL")
	);
	assert_eq!(
		substances_one[1].substance_termid_version.as_deref(),
		Some("12JUL2013")
	);
	assert_eq!(substances_one[1].strength_value, Some(decimal("3")));
	assert_eq!(substances_one[1].strength_unit.as_deref(), Some("mg"));

	assert_eq!(indications_one.len(), 2);
	assert_eq!(indications_one[0].drug_id, drugs[0].id);
	assert_eq!(indications_one[0].sequence_number, 1);
	assert_eq!(
		indications_one[0].indication_text.as_deref(),
		Some("Medication was prescribed to address sickness to the stomach.")
	);
	assert_eq!(
		indications_one[0].indication_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		indications_one[0].indication_meddra_code.as_deref(),
		Some("10028813")
	);
	assert_eq!(indications_one[1].drug_id, drugs[0].id);
	assert_eq!(indications_one[1].sequence_number, 2);
	assert_eq!(indications_one[1].indication_text, None);
	assert_eq!(
		indications_one[1].indication_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		indications_one[1].indication_meddra_code.as_deref(),
		Some("10047340")
	);

	assert_eq!(characteristics_one.len(), 10);
	assert_eq!(characteristics_one[0].drug_id, drugs[0].id);
	assert_eq!(characteristics_one[0].sequence_number, 1);
	assert_eq!(characteristics_one[0].code.as_deref(), Some("C94031"));
	assert_eq!(
		characteristics_one[0].code_system.as_deref(),
		Some("2.16.840.1.113883.3.26.1.1")
	);
	assert_eq!(
		characteristics_one[0].code_display_name.as_deref(),
		Some("FDA Specialized Product Category")
	);
	assert_eq!(characteristics_one[0].value_type.as_deref(), Some("CE"));
	assert_eq!(characteristics_one[0].value_value, None);
	assert_eq!(
		characteristics_one[0].value_code.as_deref(),
		Some("C102835")
	);
	assert_eq!(
		characteristics_one[0].value_code_system.as_deref(),
		Some("2.16.840.1.113883.3.26.1.1")
	);
	assert_eq!(
		characteristics_one[0].value_display_name.as_deref(),
		Some("Type 2: Prefilled Drug Delivery Device/System (syringe, patch, etc.)")
	);

	assert_eq!(characteristics_one[1].sequence_number, 2);
	assert_eq!(characteristics_one[1].code.as_deref(), Some("C54026"));
	assert_eq!(
		characteristics_one[1].code_display_name.as_deref(),
		Some("Malfunction")
	);
	assert_eq!(characteristics_one[1].value_type.as_deref(), Some("BL"));
	assert_eq!(characteristics_one[1].value_value.as_deref(), Some("true"));

	let last_characteristic = &characteristics_one[9];
	assert_eq!(last_characteristic.sequence_number, 10);
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

#[serial]
#[tokio::test]
async fn imports_g_linkage_models() {
	let imported_c6 = import_fixture("FAERS2022Scenario6.xml").await;
	let drugs_c6: Vec<DrugInformation> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM drug_information WHERE case_id = $1 ORDER BY sequence_number",
		imported_c6.case_id,
	)
	.await;
	let recurrences: Vec<DrugRecurrenceInformation> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM drug_recurrence_information WHERE drug_id = $1 ORDER BY sequence_number",
		drugs_c6[0].id,
	)
	.await;

	assert_eq!(recurrences.len(), 2);
	assert_eq!(recurrences[0].drug_id, drugs_c6[0].id);
	assert_eq!(recurrences[0].sequence_number, 1);
	assert_eq!(recurrences[0].rechallenge_action, None);
	assert_eq!(recurrences[0].reaction_meddra_version, None);
	assert_eq!(recurrences[0].reaction_meddra_code, None);
	assert_eq!(recurrences[0].reaction_recurred.as_deref(), Some("2"));

	assert_eq!(recurrences[1].drug_id, drugs_c6[0].id);
	assert_eq!(recurrences[1].sequence_number, 2);
	assert_eq!(recurrences[1].rechallenge_action, None);
	assert_eq!(recurrences[1].reaction_meddra_version, None);
	assert_eq!(recurrences[1].reaction_meddra_code, None);
	assert_eq!(recurrences[1].reaction_recurred.as_deref(), Some("1"));

	let imported_c1 = import_fixture("FAERS2022Scenario1.xml").await;
	let drugs_c1: Vec<DrugInformation> = list_by_uuid(
		&imported_c1,
		"SELECT * FROM drug_information WHERE case_id = $1 ORDER BY sequence_number",
		imported_c1.case_id,
	)
	.await;
	let assessments_for_drug_one: Vec<DrugReactionAssessment> = list_by_uuid(
		&imported_c1,
		"SELECT * FROM drug_reaction_assessments WHERE drug_id = $1 ORDER BY reaction_id",
		drugs_c1[0].id,
	)
	.await;
	assert_eq!(assessments_for_drug_one.len(), 2);
	assert_eq!(assessments_for_drug_one[0].drug_id, drugs_c1[0].id);
	assert_eq!(
		assessments_for_drug_one[0].administration_start_interval_value,
		None
	);
	assert_eq!(
		assessments_for_drug_one[0].administration_start_interval_unit,
		None
	);
	assert_eq!(assessments_for_drug_one[0].last_dose_interval_value, None);
	assert_eq!(assessments_for_drug_one[0].last_dose_interval_unit, None);
	assert_eq!(assessments_for_drug_one[0].recurrence_action, None);
	assert_eq!(assessments_for_drug_one[0].recurrence_meddra_version, None);
	assert_eq!(assessments_for_drug_one[0].recurrence_meddra_code, None);
	assert_eq!(assessments_for_drug_one[0].reaction_recurred, None);

	assert_eq!(assessments_for_drug_one[1].drug_id, drugs_c1[0].id);
	assert_eq!(
		assessments_for_drug_one[1].administration_start_interval_value,
		None
	);
	assert_eq!(
		assessments_for_drug_one[1].administration_start_interval_unit,
		None
	);
	assert_eq!(assessments_for_drug_one[1].last_dose_interval_value, None);
	assert_eq!(assessments_for_drug_one[1].last_dose_interval_unit, None);
	assert_eq!(assessments_for_drug_one[1].recurrence_action, None);
	assert_eq!(assessments_for_drug_one[1].recurrence_meddra_version, None);
	assert_eq!(assessments_for_drug_one[1].recurrence_meddra_code, None);
	assert_eq!(assessments_for_drug_one[1].reaction_recurred, None);
	let related: Vec<RelatednessAssessment> = list_by_uuid(
		&imported_c1,
		"SELECT * FROM relatedness_assessments WHERE drug_reaction_assessment_id IN (SELECT id FROM drug_reaction_assessments WHERE drug_id IN (SELECT id FROM drug_information WHERE case_id = $1))",
		imported_c1.case_id,
	)
	.await;
	assert_eq!(related.len(), 4);
	let mut related_summary = related
		.into_iter()
		.map(|row| {
			(
				row.sequence_number,
				row.source_of_assessment,
				row.method_of_assessment,
				row.result_of_assessment,
				row.result_of_assessment_kr2,
			)
		})
		.collect::<Vec<_>>();
	related_summary.sort();
	assert_eq!(
		related_summary,
		vec![
			(
				1,
				Some("PHARMACEUTICAL COMPANY".to_string()),
				Some("Global Introspection".to_string()),
				Some("Not Suspected".to_string()),
				None,
			),
			(
				1,
				Some("PHARMACEUTICAL COMPANY".to_string()),
				Some("Global Introspection".to_string()),
				Some("Not Suspected".to_string()),
				None,
			),
			(
				1,
				Some("PRIMARY SOURCE REPORTER".to_string()),
				Some("Global Introspection".to_string()),
				Some("Not Suspected".to_string()),
				None,
			),
			(
				1,
				Some("PRIMARY SOURCE REPORTER".to_string()),
				Some("Global Introspection".to_string()),
				Some("Suspected".to_string()),
				None,
			),
		]
	);
}

fn assert_drug_one(imported: &crate::common::ImportedCase, drug: &DrugInformation) {
	assert_eq!(drug.case_id, imported.case_id);
	assert_eq!(drug.sequence_number, 1);
	assert_eq!(drug.drug_characterization, "1");
	assert_eq!(drug.medicinal_product, "Drug A");
	assert_eq!(drug.mpid.as_deref(), Some("894444-28525-765"));
	assert_eq!(drug.mpid_version.as_deref(), Some("2014110112"));
	assert_eq!(drug.phpid, None);
	assert_eq!(drug.phpid_version, None);
	assert_eq!(drug.investigational_product_blinded, None);
	assert_eq!(drug.obtain_drug_country.as_deref(), Some("US"));
	assert_eq!(drug.brand_name, None);
	assert_eq!(drug.drug_generic_name, None);
	assert_eq!(drug.drug_authorization_number, None);
	assert_eq!(
		drug.manufacturer_name.as_deref(),
		Some("Pharmacia and Upjohn Company")
	);
	assert_eq!(drug.manufacturer_country.as_deref(), Some("US"));
	assert_eq!(drug.batch_lot_number, None);
	assert_eq!(
		drug.cumulative_dose_first_reaction_value,
		Some(decimal("150"))
	);
	assert_eq!(
		drug.cumulative_dose_first_reaction_unit.as_deref(),
		Some("mg")
	);
	assert_eq!(drug.gestation_period_exposure_value, Some(decimal("10")));
	assert_eq!(drug.gestation_period_exposure_unit.as_deref(), Some("wk"));
	assert_eq!(drug.dosage_text, None);
	assert_eq!(drug.action_taken.as_deref(), Some("2"));
	assert_eq!(drug.rechallenge.as_deref(), Some("2"));
	assert_eq!(drug.parent_route, None);
	assert_eq!(drug.parent_route_termid, None);
	assert_eq!(drug.parent_route_termid_version, None);
	assert_eq!(
		drug.parent_dosage_text.as_deref(),
		Some("Somthing seemed strange about the way it went down.")
	);
	assert_eq!(drug.fda_additional_info_coded.as_deref(), Some("4"));
	assert_eq!(drug.drug_additional_info_codes_json, None);
	assert_eq!(
		drug.fda_specialized_product_category.as_deref(),
		Some("C102835")
	);
	let device_info = parse_fda_device_info_json(drug.fda_device_info_json.as_ref())
		.expect("expected fda device info json");
	assert_eq!(device_info.malfunction, Some(true));
	assert_eq!(device_info.follow_up_types.len(), 2);
	assert_eq!(
		device_info.follow_up_types[0].value_code.as_deref(),
		Some("2")
	);
	assert_eq!(
		device_info.follow_up_types[1].value_code.as_deref(),
		Some("4")
	);
	assert_eq!(device_info.device_problem_codes.len(), 2);
	assert_eq!(
		device_info.device_problem_codes[0].value_code.as_deref(),
		Some("4001")
	);
	assert_eq!(
		device_info.device_problem_codes[1].value_code.as_deref(),
		Some("3003")
	);
	assert_eq!(device_info.device_brand_name.as_deref(), Some("Brand Name"));
	assert_eq!(
		device_info.common_device_name.as_deref(),
		Some("Common Device Name")
	);
	assert_eq!(device_info.device_product_code.as_deref(), Some("FMF"));
	assert_eq!(
		device_info.manufacturer_name.as_deref(),
		Some("Manufacturer Name")
	);
	assert_eq!(
		device_info.manufacturer_address.as_deref(),
		Some("Manufacturer Address 123 google home drive")
	);
	assert_eq!(
		device_info.manufacturer_city.as_deref(),
		Some("Manufacturer City")
	);
	assert_eq!(
		device_info.manufacturer_state.as_deref(),
		Some("Manufacturer State")
	);
	assert_eq!(device_info.manufacturer_country.as_deref(), Some("US"));
	assert_eq!(device_info.device_usage.as_deref(), Some("2"));
	assert_eq!(
		device_info.device_lot_number.as_deref(),
		Some("4577BN2-product-2-device1")
	);
	assert_eq!(device_info.operator_of_device.as_deref(), Some("1"));
	assert_eq!(device_info.remedial_actions.len(), 2);
	assert_eq!(
		device_info.remedial_actions[0].value_code.as_deref(),
		Some("6")
	);
	assert_eq!(
		device_info.remedial_actions[1].value_code.as_deref(),
		Some("5")
	);
}

fn assert_drug_two(imported: &crate::common::ImportedCase, drug: &DrugInformation) {
	assert_eq!(drug.case_id, imported.case_id);
	assert_eq!(drug.sequence_number, 2);
	assert_eq!(drug.drug_characterization, "1");
	assert_eq!(drug.medicinal_product, "Drug B");
	assert_eq!(drug.mpid.as_deref(), Some("37808-0031"));
	assert_eq!(drug.mpid_version.as_deref(), Some("2014010110"));
	assert_eq!(drug.phpid, None);
	assert_eq!(drug.phpid_version, None);
	assert_eq!(drug.investigational_product_blinded, None);
	assert_eq!(drug.obtain_drug_country, None);
	assert_eq!(drug.brand_name, None);
	assert_eq!(drug.drug_generic_name, None);
	assert_eq!(drug.drug_authorization_number, None);
	assert_eq!(drug.manufacturer_name.as_deref(), Some("Big Biopharma"));
	assert_eq!(drug.manufacturer_country.as_deref(), Some("US"));
	assert_eq!(drug.batch_lot_number, None);
	assert_eq!(drug.cumulative_dose_first_reaction_value, None);
	assert_eq!(drug.cumulative_dose_first_reaction_unit, None);
	assert_eq!(drug.gestation_period_exposure_value, None);
	assert_eq!(drug.gestation_period_exposure_unit, None);
	assert_eq!(drug.dosage_text, None);
	assert_eq!(drug.action_taken, None);
	assert_eq!(drug.rechallenge, None);
	assert_eq!(drug.parent_route, None);
	assert_eq!(drug.parent_route_termid, None);
	assert_eq!(drug.parent_route_termid_version, None);
	assert_eq!(drug.parent_dosage_text, None);
	assert_eq!(drug.fda_additional_info_coded, None);
	assert_eq!(drug.drug_additional_info_codes_json, None);
	assert_eq!(drug.fda_specialized_product_category, None);
	assert_eq!(drug.fda_device_info_json, None);
}
