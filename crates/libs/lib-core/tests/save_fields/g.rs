use super::common::{create_drug, create_reaction, date, dec, finish, setup_case};
use crate::test_common::Result;
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationForCreate, DosageInformationForUpdate,
	DrugActiveSubstanceBmc, DrugActiveSubstanceForCreate,
	DrugActiveSubstanceForUpdate, DrugDeviceCharacteristicBmc,
	DrugDeviceCharacteristicForCreate, DrugDeviceCharacteristicForUpdate,
	DrugIndicationBmc, DrugIndicationForCreate, DrugIndicationForUpdate,
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	DrugReactionAssessmentForUpdate, RelatednessAssessmentBmc,
	RelatednessAssessmentForCreate, RelatednessAssessmentForUpdate,
};
use lib_core::model::drug_recurrence::{
	DrugRecurrenceInformationBmc, DrugRecurrenceInformationForCreate,
	DrugRecurrenceInformationForUpdate,
};
use serde_json::json;
use serial_test::serial;
use time::Month;

#[tokio::test]
#[serial]
async fn save_g_k_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = DrugInformationBmc::create(
		&ctx,
		&mm,
		DrugInformationForCreate {
			case_id,
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Drug".to_string(),
			drug_generic_name: Some("Generic".to_string()),
			..Default::default()
		},
	)
	.await?;
	let row = DrugInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.drug_characterization, "1");
	assert_eq!(row.medicinal_product, "Drug");
	assert_eq!(row.mpid, None);
	assert_eq!(row.mpid_version, None);
	assert_eq!(row.phpid, None);
	assert_eq!(row.phpid_version, None);
	assert_eq!(row.investigational_product_blinded, None);
	assert_eq!(row.obtain_drug_country, None);
	assert_eq!(row.brand_name, None);
	assert_eq!(row.drug_generic_name.as_deref(), Some("Generic"));
	assert_eq!(row.drug_authorization_number, None);
	assert_eq!(row.manufacturer_name, None);
	assert_eq!(row.manufacturer_country, None);
	assert_eq!(row.batch_lot_number, None);
	assert_eq!(row.cumulative_dose_first_reaction_value, None);
	assert_eq!(row.cumulative_dose_first_reaction_unit, None);
	assert_eq!(row.gestation_period_exposure_value, None);
	assert_eq!(row.gestation_period_exposure_unit, None);
	assert_eq!(row.dosage_text, None);
	assert_eq!(row.action_taken, None);
	assert_eq!(row.rechallenge, None);
	assert_eq!(row.parent_route, None);
	assert_eq!(row.parent_route_termid, None);
	assert_eq!(row.parent_route_termid_version, None);
	assert_eq!(row.parent_dosage_text, None);
	assert_eq!(row.fda_additional_info_coded, None);
	assert_eq!(row.drug_additional_info_codes_json, None);
	assert_eq!(row.fda_specialized_product_category, None);
	assert_eq!(row.fda_device_info_json, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_create_with_top_level_identifiers() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = DrugInformationBmc::create(
		&ctx,
		&mm,
		DrugInformationForCreate {
			case_id,
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Drug".to_string(),
			drug_generic_name: Some("Generic".to_string()),
			investigational_product_blinded: Some(false),
			mpid: Some("MPID".to_string()),
			mpid_version: Some("1".to_string()),
			phpid: Some("PHPID".to_string()),
			phpid_version: Some("2".to_string()),
			obtain_drug_country: Some("US".to_string()),
			drug_authorization_number: Some("AUTH".to_string()),
			manufacturer_name: Some("Maker".to_string()),
			manufacturer_country: Some("KR".to_string()),
			batch_lot_number: Some("LOT".to_string()),
			action_taken: Some("1".to_string()),
			rechallenge: Some("2".to_string()),
			fda_additional_info_coded: Some("1".to_string()),
			drug_additional_information: Some("Additional information".to_string()),
			fda_specialized_product_category: Some("device".to_string()),
			..Default::default()
		},
	)
	.await?;
	let row = DrugInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.investigational_product_blinded, Some(false));
	assert_eq!(row.mpid.as_deref(), Some("MPID"));
	assert_eq!(row.mpid_version.as_deref(), Some("1"));
	assert_eq!(row.phpid.as_deref(), Some("PHPID"));
	assert_eq!(row.phpid_version.as_deref(), Some("2"));
	assert_eq!(row.obtain_drug_country.as_deref(), Some("US"));
	assert_eq!(row.drug_authorization_number.as_deref(), Some("AUTH"));
	assert_eq!(row.manufacturer_name.as_deref(), Some("Maker"));
	assert_eq!(row.manufacturer_country.as_deref(), Some("KR"));
	assert_eq!(row.batch_lot_number.as_deref(), Some("LOT"));
	assert_eq!(row.action_taken.as_deref(), Some("1"));
	assert_eq!(row.rechallenge.as_deref(), Some("2"));
	assert_eq!(row.fda_additional_info_coded.as_deref(), Some("1"));
	assert_eq!(
		row.drug_additional_information.as_deref(),
		Some("Additional information")
	);
	assert_eq!(
		row.fda_specialized_product_category.as_deref(),
		Some("device")
	);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = create_drug(&ctx, &mm, case_id).await?;
	DrugInformationBmc::update_in_case(
		&ctx,
		&mm,
		case_id,
		id,
		DrugInformationForUpdate {
			medicinal_product: Some("Drug 2".to_string()),
			drug_characterization: Some("2".to_string()),
			brand_name: Some("Brand".to_string()),
			drug_generic_name: Some("Generic".to_string()),
			drug_authorization_number: Some("AUTH".to_string()),
			manufacturer_name: Some("Maker".to_string()),
			manufacturer_country: Some("KR".to_string()),
			batch_lot_number: Some("LOT".to_string()),
			cumulative_dose_first_reaction_value: Some(dec(150, 0)),
			cumulative_dose_first_reaction_unit: Some("mg".to_string()),
			gestation_period_exposure_value: Some(dec(10, 0)),
			gestation_period_exposure_unit: Some("wk".to_string()),
			dosage_text: Some("Dosage".to_string()),
			action_taken: Some("1".to_string()),
			rechallenge: Some("2".to_string()),
			investigational_product_blinded: Some(false),
			mpid: Some("MPID".to_string()),
			mpid_version: Some("1".to_string()),
			phpid: Some("PHPID".to_string()),
			phpid_version: Some("2".to_string()),
			obtain_drug_country: Some("US".to_string()),
			parent_route: Some("oral".to_string()),
			parent_route_termid: Some("001".to_string()),
			parent_route_termid_version: Some("1".to_string()),
			parent_dosage_text: Some("Parent dose".to_string()),
			fda_additional_info_coded: Some("1".to_string()),
			drug_additional_info_codes_json: Some(json!(["A", "B"])),
			drug_additional_information: Some("Additional information".to_string()),
			fda_specialized_product_category: Some("device".to_string()),
			fda_device_info_json: Some(json!({"device":"x"})),
		},
	)
	.await?;
	let row = DrugInformationBmc::get_in_case(&ctx, &mm, case_id, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.drug_characterization, "2");
	assert_eq!(row.medicinal_product, "Drug 2");
	assert_eq!(row.mpid.as_deref(), Some("MPID"));
	assert_eq!(row.mpid_version.as_deref(), Some("1"));
	assert_eq!(row.phpid.as_deref(), Some("PHPID"));
	assert_eq!(row.phpid_version.as_deref(), Some("2"));
	assert_eq!(row.investigational_product_blinded, Some(false));
	assert_eq!(row.obtain_drug_country.as_deref(), Some("US"));
	assert_eq!(row.brand_name.as_deref(), Some("Brand"));
	assert_eq!(row.drug_generic_name.as_deref(), Some("Generic"));
	assert_eq!(row.drug_authorization_number.as_deref(), Some("AUTH"));
	assert_eq!(row.manufacturer_name.as_deref(), Some("Maker"));
	assert_eq!(row.manufacturer_country.as_deref(), Some("KR"));
	assert_eq!(row.batch_lot_number.as_deref(), Some("LOT"));
	assert_eq!(row.cumulative_dose_first_reaction_value, Some(dec(150, 0)));
	assert_eq!(
		row.cumulative_dose_first_reaction_unit.as_deref(),
		Some("mg")
	);
	assert_eq!(row.gestation_period_exposure_value, Some(dec(10, 0)));
	assert_eq!(row.gestation_period_exposure_unit.as_deref(), Some("wk"));
	assert_eq!(row.dosage_text.as_deref(), Some("Dosage"));
	assert_eq!(row.action_taken.as_deref(), Some("1"));
	assert_eq!(row.rechallenge.as_deref(), Some("2"));
	assert_eq!(row.parent_route.as_deref(), Some("oral"));
	assert_eq!(row.parent_route_termid.as_deref(), Some("001"));
	assert_eq!(row.parent_route_termid_version.as_deref(), Some("1"));
	assert_eq!(row.parent_dosage_text.as_deref(), Some("Parent dose"));
	assert_eq!(row.fda_additional_info_coded.as_deref(), Some("1"));
	assert_eq!(row.drug_additional_info_codes_json, Some(json!(["A", "B"])));
	assert_eq!(
		row.drug_additional_information.as_deref(),
		Some("Additional information")
	);
	assert_eq!(
		row.fda_specialized_product_category.as_deref(),
		Some("device")
	);
	assert_eq!(row.fda_device_info_json, Some(json!({"device":"x"})));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_2_3_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Substance".to_string()),
			substance_termid: Some("S1".to_string()),
			substance_termid_version: Some("1".to_string()),
			strength_value: Some(dec(1, 0)),
			strength_unit: Some("mg".to_string()),
		},
	)
	.await?;
	let row = DrugActiveSubstanceBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.substance_name.as_deref(), Some("Substance"));
	assert_eq!(row.substance_termid.as_deref(), Some("S1"));
	assert_eq!(row.substance_termid_version.as_deref(), Some("1"));
	assert_eq!(row.strength_value, Some(dec(1, 0)));
	assert_eq!(row.strength_unit.as_deref(), Some("mg"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_2_3_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: None,
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;
	DrugActiveSubstanceBmc::update(
		&ctx,
		&mm,
		id,
		DrugActiveSubstanceForUpdate {
			substance_name: Some("Substance 2".to_string()),
			substance_termid: Some("S2".to_string()),
			substance_termid_version: Some("2".to_string()),
			strength_value: Some(dec(2, 0)),
			strength_unit: Some("g".to_string()),
		},
	)
	.await?;
	let row = DrugActiveSubstanceBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.substance_name.as_deref(), Some("Substance 2"));
	assert_eq!(row.substance_termid.as_deref(), Some("S2"));
	assert_eq!(row.substance_termid_version.as_deref(), Some("2"));
	assert_eq!(row.strength_value, Some(dec(2, 0)));
	assert_eq!(row.strength_unit.as_deref(), Some("g"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_4_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DosageInformationBmc::create(
		&ctx,
		&mm,
		DosageInformationForCreate {
			drug_id,
			sequence_number: 1,
			dose_value: Some(dec(1, 0)),
			dose_unit: Some("mg".to_string()),
			number_of_units: Some(2),
			frequency_value: Some(dec(1, 0)),
			frequency_unit: Some("d".to_string()),
			first_administration_date: Some(date(2024, Month::January, 1)),
			first_administration_time: Some(
				sqlx::types::time::Time::from_hms(8, 0, 0).unwrap(),
			),
			last_administration_date: Some(date(2024, Month::January, 2)),
			last_administration_time: Some(
				sqlx::types::time::Time::from_hms(9, 0, 0).unwrap(),
			),
			duration_value: Some(dec(2, 0)),
			duration_unit: Some("d".to_string()),
			continuing: None,
			batch_lot_number: Some("LOT".to_string()),
			dosage_text: Some("Dose".to_string()),
			dose_form: Some("Tablet".to_string()),
			dose_form_termid: Some("DF1".to_string()),
			dose_form_termid_version: Some("1".to_string()),
			route_of_administration: Some("PO".to_string()),
			route_termid: Some("RO1".to_string()),
			route_termid_version: Some("1".to_string()),
			parent_route: Some("oral".to_string()),
			parent_route_termid: Some("001".to_string()),
			parent_route_termid_version: Some("1".to_string()),
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
		},
	)
	.await?;
	let row = DosageInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.dose_value, Some(dec(1, 0)));
	assert_eq!(row.dose_unit.as_deref(), Some("mg"));
	assert_eq!(row.number_of_units, Some(2));
	assert_eq!(row.frequency_value, Some(dec(1, 0)));
	assert_eq!(row.frequency_unit.as_deref(), Some("d"));
	assert_eq!(
		row.first_administration_date,
		Some(date(2024, Month::January, 1))
	);
	assert_eq!(
		row.first_administration_time,
		Some(sqlx::types::time::Time::from_hms(8, 0, 0).unwrap())
	);
	assert_eq!(
		row.last_administration_date,
		Some(date(2024, Month::January, 2))
	);
	assert_eq!(
		row.last_administration_time,
		Some(sqlx::types::time::Time::from_hms(9, 0, 0).unwrap())
	);
	assert_eq!(row.duration_value, Some(dec(2, 0)));
	assert_eq!(row.duration_unit.as_deref(), Some("d"));
	assert_eq!(row.continuing, None);
	assert_eq!(row.batch_lot_number.as_deref(), Some("LOT"));
	assert_eq!(row.dosage_text.as_deref(), Some("Dose"));
	assert_eq!(row.dose_form.as_deref(), Some("Tablet"));
	assert_eq!(row.dose_form_termid.as_deref(), Some("DF1"));
	assert_eq!(row.dose_form_termid_version.as_deref(), Some("1"));
	assert_eq!(row.route_of_administration.as_deref(), Some("PO"));
	assert_eq!(row.route_termid.as_deref(), Some("RO1"));
	assert_eq!(row.route_termid_version.as_deref(), Some("1"));
	assert_eq!(row.parent_route.as_deref(), Some("oral"));
	assert_eq!(row.parent_route_termid.as_deref(), Some("001"));
	assert_eq!(row.parent_route_termid_version.as_deref(), Some("1"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_4_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DosageInformationBmc::create(
		&ctx,
		&mm,
		DosageInformationForCreate {
			drug_id,
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_value: None,
			frequency_unit: None,
			first_administration_date: None,
			first_administration_time: None,
			last_administration_date: None,
			last_administration_time: None,
			duration_value: None,
			duration_unit: None,
			continuing: None,
			batch_lot_number: None,
			dosage_text: None,
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: None,
			route_termid: None,
			route_termid_version: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			first_administration_date_null_flavor: Some("NI".to_string()),
			last_administration_date_null_flavor: Some("UNK".to_string()),
		},
	)
	.await?;
	DosageInformationBmc::update(
		&ctx,
		&mm,
		id,
		DosageInformationForUpdate {
			dose_value: Some(dec(2, 0)),
			dose_unit: Some("g".to_string()),
			number_of_units: Some(3),
			frequency_value: Some(dec(2, 0)),
			frequency_unit: Some("wk".to_string()),
			first_administration_date: Some(date(2024, Month::February, 1)),
			first_administration_time: Some(
				sqlx::types::time::Time::from_hms(10, 0, 0).unwrap(),
			),
			last_administration_date: Some(date(2024, Month::February, 2)),
			last_administration_time: Some(
				sqlx::types::time::Time::from_hms(11, 0, 0).unwrap(),
			),
			duration_value: Some(dec(3, 0)),
			duration_unit: Some("wk".to_string()),
			continuing: Some(true),
			batch_lot_number: Some("LOT2".to_string()),
			dosage_text: Some("Dose 2".to_string()),
			dose_form: Some("Capsule".to_string()),
			dose_form_termid: Some("DF2".to_string()),
			dose_form_termid_version: Some("2".to_string()),
			route_of_administration: Some("IV".to_string()),
			route_termid: Some("RO2".to_string()),
			route_termid_version: Some("2".to_string()),
			parent_route: Some("iv".to_string()),
			parent_route_termid: Some("002".to_string()),
			parent_route_termid_version: Some("2".to_string()),
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
		},
	)
	.await?;
	let row = DosageInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.dose_value, Some(dec(2, 0)));
	assert_eq!(row.dose_unit.as_deref(), Some("g"));
	assert_eq!(row.number_of_units, Some(3));
	assert_eq!(row.frequency_value, Some(dec(2, 0)));
	assert_eq!(row.frequency_unit.as_deref(), Some("wk"));
	assert_eq!(
		row.first_administration_date,
		Some(date(2024, Month::February, 1))
	);
	assert_eq!(
		row.first_administration_time,
		Some(sqlx::types::time::Time::from_hms(10, 0, 0).unwrap())
	);
	assert_eq!(
		row.last_administration_date,
		Some(date(2024, Month::February, 2))
	);
	assert_eq!(
		row.last_administration_time,
		Some(sqlx::types::time::Time::from_hms(11, 0, 0).unwrap())
	);
	assert_eq!(row.duration_value, Some(dec(3, 0)));
	assert_eq!(row.duration_unit.as_deref(), Some("wk"));
	assert_eq!(row.continuing, Some(true));
	assert_eq!(row.batch_lot_number.as_deref(), Some("LOT2"));
	assert_eq!(row.dosage_text.as_deref(), Some("Dose 2"));
	assert_eq!(row.dose_form.as_deref(), Some("Capsule"));
	assert_eq!(row.dose_form_termid.as_deref(), Some("DF2"));
	assert_eq!(row.dose_form_termid_version.as_deref(), Some("2"));
	assert_eq!(row.route_of_administration.as_deref(), Some("IV"));
	assert_eq!(row.route_termid.as_deref(), Some("RO2"));
	assert_eq!(row.route_termid_version.as_deref(), Some("2"));
	assert_eq!(row.parent_route.as_deref(), Some("iv"));
	assert_eq!(row.parent_route_termid.as_deref(), Some("002"));
	assert_eq!(row.parent_route_termid_version.as_deref(), Some("2"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_6_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DrugIndicationBmc::create(
		&ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id,
			sequence_number: 1,
			indication_text: Some("Indication".to_string()),
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: Some("900".to_string()),
		},
	)
	.await?;
	let row = DrugIndicationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.indication_text.as_deref(), Some("Indication"));
	assert_eq!(row.indication_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.indication_meddra_code.as_deref(), Some("900"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_6_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DrugIndicationBmc::create(
		&ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id,
			sequence_number: 1,
			indication_text: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
		},
	)
	.await?;
	DrugIndicationBmc::update(
		&ctx,
		&mm,
		id,
		DrugIndicationForUpdate {
			indication_text: Some("Indication 2".to_string()),
			indication_meddra_version: Some("28.0".to_string()),
			indication_meddra_code: Some("901".to_string()),
		},
	)
	.await?;
	let row = DrugIndicationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.indication_text.as_deref(), Some("Indication 2"));
	assert_eq!(row.indication_meddra_version.as_deref(), Some("28.0"));
	assert_eq!(row.indication_meddra_code.as_deref(), Some("901"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_8_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DrugRecurrenceInformationBmc::create(
		&ctx,
		&mm,
		DrugRecurrenceInformationForCreate {
			drug_id,
			sequence_number: 1,
			rechallenge_action: Some("1".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			reaction_meddra_code: Some("100".to_string()),
			reaction_recurred: Some("2".to_string()),
		},
	)
	.await?;
	let row = DrugRecurrenceInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.rechallenge_action.as_deref(), Some("1"));
	assert_eq!(row.reaction_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.reaction_meddra_code.as_deref(), Some("100"));
	assert_eq!(row.reaction_recurred.as_deref(), Some("2"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_8_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DrugRecurrenceInformationBmc::create(
		&ctx,
		&mm,
		DrugRecurrenceInformationForCreate {
			drug_id,
			sequence_number: 1,
			rechallenge_action: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			reaction_recurred: None,
		},
	)
	.await?;
	DrugRecurrenceInformationBmc::update(
		&ctx,
		&mm,
		id,
		DrugRecurrenceInformationForUpdate {
			rechallenge_action: Some("1".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			reaction_meddra_code: Some("100".to_string()),
			reaction_recurred: Some("2".to_string()),
		},
	)
	.await?;
	let row = DrugRecurrenceInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.rechallenge_action.as_deref(), Some("1"));
	assert_eq!(row.reaction_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.reaction_meddra_code.as_deref(), Some("100"));
	assert_eq!(row.reaction_recurred.as_deref(), Some("2"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_10_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		DrugDeviceCharacteristicForCreate {
			drug_id,
			sequence_number: 1,
			code: Some("C1".to_string()),
			code_system: Some("CS".to_string()),
			code_display_name: Some("Device".to_string()),
			value_type: Some("CE".to_string()),
			value_value: Some("Value".to_string()),
			value_code: Some("VC1".to_string()),
			value_code_system: Some("VCS".to_string()),
			value_display_name: Some("VD".to_string()),
		},
	)
	.await?;
	let row = DrugDeviceCharacteristicBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.code.as_deref(), Some("C1"));
	assert_eq!(row.code_system.as_deref(), Some("CS"));
	assert_eq!(row.code_display_name.as_deref(), Some("Device"));
	assert_eq!(row.value_type.as_deref(), Some("CE"));
	assert_eq!(row.value_value.as_deref(), Some("Value"));
	assert_eq!(row.value_code.as_deref(), Some("VC1"));
	assert_eq!(row.value_code_system.as_deref(), Some("VCS"));
	assert_eq!(row.value_display_name.as_deref(), Some("VD"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_10_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let id = DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		DrugDeviceCharacteristicForCreate {
			drug_id,
			sequence_number: 1,
			code: None,
			code_system: None,
			code_display_name: None,
			value_type: None,
			value_value: None,
			value_code: None,
			value_code_system: None,
			value_display_name: None,
		},
	)
	.await?;
	DrugDeviceCharacteristicBmc::update(
		&ctx,
		&mm,
		id,
		DrugDeviceCharacteristicForUpdate {
			code: Some("C2".to_string()),
			code_system: Some("CS2".to_string()),
			code_display_name: Some("Device 2".to_string()),
			value_type: Some("ST".to_string()),
			value_value: Some("Value 2".to_string()),
			value_code: Some("VC2".to_string()),
			value_code_system: Some("VCS2".to_string()),
			value_display_name: Some("VD2".to_string()),
		},
	)
	.await?;
	let row = DrugDeviceCharacteristicBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.code.as_deref(), Some("C2"));
	assert_eq!(row.code_system.as_deref(), Some("CS2"));
	assert_eq!(row.code_display_name.as_deref(), Some("Device 2"));
	assert_eq!(row.value_type.as_deref(), Some("ST"));
	assert_eq!(row.value_value.as_deref(), Some("Value 2"));
	assert_eq!(row.value_code.as_deref(), Some("VC2"));
	assert_eq!(row.value_code_system.as_deref(), Some("VCS2"));
	assert_eq!(row.value_display_name.as_deref(), Some("VD2"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_9_i_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let reaction_id = create_reaction(&ctx, &mm, case_id).await?;
	let id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
			administration_start_interval_value: Some(dec(2, 0)),
			administration_start_interval_unit: Some("d".to_string()),
			last_dose_interval_value: Some(dec(1, 0)),
			last_dose_interval_unit: Some("h".to_string()),
			recurrence_action: Some("3".to_string()),
			recurrence_meddra_version: Some("27.0".to_string()),
			recurrence_meddra_code: Some("100".to_string()),
			reaction_recurred: Some("1".to_string()),
		},
	)
	.await?;
	let row = DrugReactionAssessmentBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.reaction_id, reaction_id);
	assert_eq!(row.administration_start_interval_value, Some(dec(2, 0)));
	assert_eq!(row.administration_start_interval_unit.as_deref(), Some("d"));
	assert_eq!(row.last_dose_interval_value, Some(dec(1, 0)));
	assert_eq!(row.last_dose_interval_unit.as_deref(), Some("h"));
	assert_eq!(row.recurrence_action.as_deref(), Some("3"));
	assert_eq!(row.recurrence_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.recurrence_meddra_code.as_deref(), Some("100"));
	assert_eq!(row.reaction_recurred.as_deref(), Some("1"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_9_i_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let reaction_id = create_reaction(&ctx, &mm, case_id).await?;
	let id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
			administration_start_interval_value: None,
			administration_start_interval_unit: None,
			last_dose_interval_value: None,
			last_dose_interval_unit: None,
			recurrence_action: None,
			recurrence_meddra_version: None,
			recurrence_meddra_code: None,
			reaction_recurred: None,
		},
	)
	.await?;
	DrugReactionAssessmentBmc::update(
		&ctx,
		&mm,
		id,
		DrugReactionAssessmentForUpdate {
			administration_start_interval_value: Some(dec(2, 0)),
			administration_start_interval_unit: Some("d".to_string()),
			last_dose_interval_value: Some(dec(1, 0)),
			last_dose_interval_unit: Some("h".to_string()),
			recurrence_action: Some("3".to_string()),
			recurrence_meddra_version: Some("27.0".to_string()),
			recurrence_meddra_code: Some("100".to_string()),
			reaction_recurred: Some("1".to_string()),
		},
	)
	.await?;
	let row = DrugReactionAssessmentBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_id, drug_id);
	assert_eq!(row.reaction_id, reaction_id);
	assert_eq!(row.administration_start_interval_value, Some(dec(2, 0)));
	assert_eq!(row.administration_start_interval_unit.as_deref(), Some("d"));
	assert_eq!(row.last_dose_interval_value, Some(dec(1, 0)));
	assert_eq!(row.last_dose_interval_unit.as_deref(), Some("h"));
	assert_eq!(row.recurrence_action.as_deref(), Some("3"));
	assert_eq!(row.recurrence_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.recurrence_meddra_code.as_deref(), Some("100"));
	assert_eq!(row.reaction_recurred.as_deref(), Some("1"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_9_i_2_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let reaction_id = create_reaction(&ctx, &mm, case_id).await?;
	let dra_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
			administration_start_interval_value: None,
			administration_start_interval_unit: None,
			last_dose_interval_value: None,
			last_dose_interval_unit: None,
			recurrence_action: None,
			recurrence_meddra_version: None,
			recurrence_meddra_code: None,
			reaction_recurred: None,
		},
	)
	.await?;
	let id = RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: dra_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: Some("WHO".to_string()),
			result_of_assessment: Some("related".to_string()),
			result_of_assessment_kr2: Some("KR".to_string()),
		},
	)
	.await?;
	let row = RelatednessAssessmentBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_reaction_assessment_id, dra_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.source_of_assessment.as_deref(), Some("Reporter"));
	assert_eq!(row.method_of_assessment.as_deref(), Some("WHO"));
	assert_eq!(row.result_of_assessment.as_deref(), Some("related"));
	// G.k.9.i.2.r.3.KR.2 is directly saveable even though XML mapping remains unsupported.
	assert_eq!(row.result_of_assessment_kr2.as_deref(), Some("KR"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_g_k_9_i_2_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let drug_id = create_drug(&ctx, &mm, case_id).await?;
	let reaction_id = create_reaction(&ctx, &mm, case_id).await?;
	let dra_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
			administration_start_interval_value: None,
			administration_start_interval_unit: None,
			last_dose_interval_value: None,
			last_dose_interval_unit: None,
			recurrence_action: None,
			recurrence_meddra_version: None,
			recurrence_meddra_code: None,
			reaction_recurred: None,
		},
	)
	.await?;
	let id = RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: dra_id,
			sequence_number: 1,
			source_of_assessment: None,
			method_of_assessment: None,
			result_of_assessment: None,
			result_of_assessment_kr2: None,
		},
	)
	.await?;
	RelatednessAssessmentBmc::update(
		&ctx,
		&mm,
		id,
		RelatednessAssessmentForUpdate {
			source_of_assessment: Some("Sponsor".to_string()),
			method_of_assessment: Some("Naranjo".to_string()),
			result_of_assessment: Some("not related".to_string()),
			result_of_assessment_kr2: Some("KR2".to_string()),
		},
	)
	.await?;
	let row = RelatednessAssessmentBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.drug_reaction_assessment_id, dra_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.source_of_assessment.as_deref(), Some("Sponsor"));
	assert_eq!(row.method_of_assessment.as_deref(), Some("Naranjo"));
	assert_eq!(row.result_of_assessment.as_deref(), Some("not related"));
	// G.k.9.i.2.r.3.KR.2 is directly saveable even though XML mapping remains unsupported.
	assert_eq!(row.result_of_assessment_kr2.as_deref(), Some("KR2"));
	finish(&mm).await
}
