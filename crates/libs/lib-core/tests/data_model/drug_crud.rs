use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationFilter, DosageInformationForCreate,
	DosageInformationForUpdate, DrugActiveSubstanceBmc, DrugActiveSubstanceFilter,
	DrugActiveSubstanceForCreate, DrugActiveSubstanceForUpdate, DrugIndicationBmc,
	DrugIndicationFilter, DrugIndicationForCreate, DrugIndicationForUpdate,
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
use modql::filter::{OpValValue, OpValsValue};
use rust_decimal::Decimal;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_drug_information_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Demo Drug".to_string(),
		drug_generic_name: None,
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;
	let drug = DrugInformationBmc::get(&ctx, &mm, drug_id).await?;
	assert_eq!(drug.medicinal_product, "Demo Drug");

	let drug_u = DrugInformationForUpdate {
		medicinal_product: Some("Updated Drug".to_string()),
		drug_characterization: None,
		brand_name: None,
		drug_generic_name: None,
		drug_authorization_number: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		cumulative_dose_first_reaction_value: None,
		cumulative_dose_first_reaction_unit: None,
		gestation_period_exposure_value: None,
		gestation_period_exposure_unit: None,
		dosage_text: None,
		action_taken: Some("1".to_string()),
		rechallenge: None,
		investigational_product_blinded: None,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		obtain_drug_country: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		parent_dosage_text: None,
		fda_additional_info_coded: None,
		drug_additional_info_codes_json: None,
		fda_specialized_product_category: None,
		fda_device_info_json: None,
	};
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;
	let drug = DrugInformationBmc::get_in_case(&ctx, &mm, case_id, drug_id).await?;
	assert_eq!(drug.medicinal_product, "Updated Drug");

	let drugs = DrugInformationBmc::list_by_case(&ctx, &mm, case_id).await?;
	assert!(drugs.iter().any(|d| d.id == drug_id));

	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_submodels_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Submodel Drug".to_string(),
		drug_generic_name: None,
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	let substance_c = DrugActiveSubstanceForCreate {
		drug_id,
		sequence_number: 1,
		substance_name: Some("Substance A".to_string()),
		substance_termid: None,
		substance_termid_version: None,
		strength_value: None,
		strength_unit: None,
	};
	let substance_id =
		DrugActiveSubstanceBmc::create(&ctx, &mm, substance_c).await?;
	let substance = DrugActiveSubstanceBmc::get(&ctx, &mm, substance_id).await?;
	assert_eq!(substance.sequence_number, 1);

	let substance_u = DrugActiveSubstanceForUpdate {
		substance_name: Some("Substance B".to_string()),
		substance_termid: None,
		substance_termid_version: None,
		strength_value: None,
		strength_unit: Some("mg".to_string()),
	};
	DrugActiveSubstanceBmc::update(&ctx, &mm, substance_id, substance_u).await?;
	let substance = DrugActiveSubstanceBmc::get(&ctx, &mm, substance_id).await?;
	assert_eq!(substance.substance_name.as_deref(), Some("Substance B"));

	let substances = DrugActiveSubstanceBmc::list(
		&ctx,
		&mm,
		Some(vec![DrugActiveSubstanceFilter {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(
				drug_id.to_string().into(),
			)])),
			sequence_number: None,
		}]),
		None,
	)
	.await?;
	assert!(substances.iter().any(|s| s.id == substance_id));

	let dosage_c = DosageInformationForCreate {
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
		batch_lot_number: None,
		dosage_text: None,
		dose_form: None,
		dose_form_termid: None,
		dose_form_termid_version: None,
		route_of_administration: None,
		route_termid_version: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
	};
	let dosage_id = DosageInformationBmc::create(&ctx, &mm, dosage_c).await?;
	let dosage = DosageInformationBmc::get(&ctx, &mm, dosage_id).await?;
	assert_eq!(dosage.sequence_number, 1);

	let dosage_u = DosageInformationForUpdate {
		dose_value: Some(Decimal::new(1, 0)),
		dose_unit: Some("tab".to_string()),
		number_of_units: None,
		frequency_value: None,
		frequency_unit: None,
		first_administration_date: None,
		first_administration_time: None,
		last_administration_date: None,
		last_administration_time: None,
		duration_value: None,
		duration_unit: None,
		batch_lot_number: None,
		dosage_text: Some("Dose text".to_string()),
		dose_form: None,
		dose_form_termid: None,
		dose_form_termid_version: None,
		route_of_administration: None,
		route_termid_version: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
	};
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;
	let dosage = DosageInformationBmc::get(&ctx, &mm, dosage_id).await?;
	assert_eq!(dosage.dose_unit.as_deref(), Some("tab"));

	let dosage_list = DosageInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![DosageInformationFilter {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(
				drug_id.to_string().into(),
			)])),
			sequence_number: None,
		}]),
		None,
	)
	.await?;
	assert!(dosage_list.iter().any(|d| d.id == dosage_id));

	let indication_c = DrugIndicationForCreate {
		drug_id,
		sequence_number: 1,
		indication_text: Some("Headache".to_string()),
		indication_meddra_version: None,
		indication_meddra_code: None,
	};
	let indication_id = DrugIndicationBmc::create(&ctx, &mm, indication_c).await?;
	let indication = DrugIndicationBmc::get(&ctx, &mm, indication_id).await?;
	assert_eq!(indication.sequence_number, 1);

	let indication_u = DrugIndicationForUpdate {
		indication_text: Some("Fever".to_string()),
		indication_meddra_version: None,
		indication_meddra_code: None,
	};
	DrugIndicationBmc::update(&ctx, &mm, indication_id, indication_u).await?;
	let indication = DrugIndicationBmc::get(&ctx, &mm, indication_id).await?;
	assert_eq!(indication.indication_text.as_deref(), Some("Fever"));

	let indications = DrugIndicationBmc::list(
		&ctx,
		&mm,
		Some(vec![DrugIndicationFilter {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(
				drug_id.to_string().into(),
			)])),
			sequence_number: None,
		}]),
		None,
	)
	.await?;
	assert!(indications.iter().any(|i| i.id == indication_id));

	DrugIndicationBmc::delete(&ctx, &mm, indication_id).await?;
	DosageInformationBmc::delete(&ctx, &mm, dosage_id).await?;
	DrugActiveSubstanceBmc::delete(&ctx, &mm, substance_id).await?;
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
