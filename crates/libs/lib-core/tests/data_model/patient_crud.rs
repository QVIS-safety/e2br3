use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathForCreate,
	AutopsyCauseOfDeathForUpdate, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeForCreate, MedicalHistoryEpisodeForUpdate,
	ParentInformationBmc, ParentInformationForCreate, ParentInformationForUpdate,
	PastDrugHistoryBmc, PastDrugHistoryForCreate, PastDrugHistoryForUpdate,
	PatientDeathInformationBmc, PatientDeathInformationForCreate,
	PatientDeathInformationForUpdate, PatientInformationBmc,
	PatientInformationForCreate, PatientInformationForUpdate,
	ReportedCauseOfDeathBmc, ReportedCauseOfDeathForCreate,
	ReportedCauseOfDeathForUpdate,
};
use rust_decimal::Decimal;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_patient_information_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let patient_c = PatientInformationForCreate {
		case_id,
		patient_initials: Some("TU".to_string()),
		sex: Some("1".to_string()),
		concomitant_therapy: None,
	};
	let patient_id = PatientInformationBmc::create(&ctx, &mm, patient_c).await?;
	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(patient.id, patient_id);

	let patient_by_id = PatientInformationBmc::get(&ctx, &mm, patient_id).await?;
	assert_eq!(patient_by_id.id, patient_id);

	let patients = PatientInformationBmc::list(&ctx, &mm, None, None).await?;
	assert!(patients.iter().any(|p| p.id == patient_id));

	let patient_u = PatientInformationForUpdate {
		patient_initials: Some("UP".to_string()),
		patient_given_name: None,
		patient_family_name: None,
		birth_date: None,
		age_at_time_of_onset: None,
		age_unit: None,
		gestation_period: None,
		gestation_period_unit: None,
		age_group: None,
		race_code: None,
		ethnicity_code: None,
		weight_kg: None,
		height_cm: None,
		sex: None,
		last_menstrual_period_date: None,
		medical_history_text: Some("Updated history".to_string()),
		concomitant_therapy: None,
	};
	PatientInformationBmc::update_by_case(&ctx, &mm, case_id, patient_u).await?;
	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(patient.patient_initials.as_deref(), Some("UP"));

	PatientInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_patient_submodels_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let patient_c = PatientInformationForCreate {
		case_id,
		patient_initials: Some("SB".to_string()),
		sex: Some("1".to_string()),
		concomitant_therapy: None,
	};
	let patient_id = PatientInformationBmc::create(&ctx, &mm, patient_c).await?;

	let med_c = MedicalHistoryEpisodeForCreate {
		patient_id,
		sequence_number: 1,
		meddra_code: Some("12345678".to_string()),
	};
	let med_id = MedicalHistoryEpisodeBmc::create(&ctx, &mm, med_c).await?;
	let med = MedicalHistoryEpisodeBmc::get(&ctx, &mm, med_id).await?;
	assert_eq!(med.sequence_number, 1);

	let med_u = MedicalHistoryEpisodeForUpdate {
		meddra_version: Some("26.0".to_string()),
		meddra_code: None,
		start_date: None,
		continuing: Some(true),
		end_date: None,
		comments: None,
		family_history: None,
	};
	MedicalHistoryEpisodeBmc::update(&ctx, &mm, med_id, med_u).await?;
	let med = MedicalHistoryEpisodeBmc::get(&ctx, &mm, med_id).await?;
	assert_eq!(med.meddra_version.as_deref(), Some("26.0"));

	let med_list = MedicalHistoryEpisodeBmc::list(&ctx, &mm, None, None).await?;
	assert!(med_list.iter().any(|m| m.id == med_id));

	let past_c = PastDrugHistoryForCreate {
		patient_id,
		sequence_number: 1,
		drug_name: Some("Old Drug".to_string()),
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		start_date: None,
		end_date: None,
		indication_meddra_version: None,
		indication_meddra_code: None,
		reaction_meddra_version: None,
		reaction_meddra_code: None,
	};
	let past_id = PastDrugHistoryBmc::create(&ctx, &mm, past_c).await?;
	let past = PastDrugHistoryBmc::get(&ctx, &mm, past_id).await?;
	assert_eq!(past.sequence_number, 1);

	let past_u = PastDrugHistoryForUpdate {
		drug_name: Some("Old Drug Updated".to_string()),
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		start_date: None,
		end_date: None,
		indication_meddra_version: None,
		indication_meddra_code: None,
		reaction_meddra_version: None,
		reaction_meddra_code: None,
	};
	PastDrugHistoryBmc::update(&ctx, &mm, past_id, past_u).await?;
	let past = PastDrugHistoryBmc::get(&ctx, &mm, past_id).await?;
	assert_eq!(past.drug_name.as_deref(), Some("Old Drug Updated"));

	let death_c = PatientDeathInformationForCreate {
		patient_id,
		date_of_death: None,
		autopsy_performed: Some(false),
	};
	let death_id = PatientDeathInformationBmc::create(&ctx, &mm, death_c).await?;
	let death = PatientDeathInformationBmc::get(&ctx, &mm, death_id).await?;
	assert_eq!(death.patient_id, patient_id);

	let death_u = PatientDeathInformationForUpdate {
		date_of_death: None,
		autopsy_performed: Some(true),
	};
	PatientDeathInformationBmc::update(&ctx, &mm, death_id, death_u).await?;
	let death = PatientDeathInformationBmc::get(&ctx, &mm, death_id).await?;
	assert_eq!(death.autopsy_performed, Some(true));

	let reported_c = ReportedCauseOfDeathForCreate {
		death_info_id: death_id,
		sequence_number: 1,
		meddra_code: Some("87654321".to_string()),
	};
	let reported_id = ReportedCauseOfDeathBmc::create(&ctx, &mm, reported_c).await?;
	let reported = ReportedCauseOfDeathBmc::get(&ctx, &mm, reported_id).await?;
	assert_eq!(reported.sequence_number, 1);

	let reported_u = ReportedCauseOfDeathForUpdate {
		meddra_version: Some("26.0".to_string()),
		meddra_code: None,
		comments: None,
	};
	ReportedCauseOfDeathBmc::update(&ctx, &mm, reported_id, reported_u).await?;
	let reported = ReportedCauseOfDeathBmc::get(&ctx, &mm, reported_id).await?;
	assert_eq!(reported.meddra_version.as_deref(), Some("26.0"));

	let autopsy_c = AutopsyCauseOfDeathForCreate {
		death_info_id: death_id,
		sequence_number: 1,
		meddra_code: Some("87654322".to_string()),
	};
	let autopsy_id = AutopsyCauseOfDeathBmc::create(&ctx, &mm, autopsy_c).await?;
	let autopsy = AutopsyCauseOfDeathBmc::get(&ctx, &mm, autopsy_id).await?;
	assert_eq!(autopsy.sequence_number, 1);

	let autopsy_u = AutopsyCauseOfDeathForUpdate {
		meddra_version: Some("26.0".to_string()),
		meddra_code: None,
		comments: None,
	};
	AutopsyCauseOfDeathBmc::update(&ctx, &mm, autopsy_id, autopsy_u).await?;
	let autopsy = AutopsyCauseOfDeathBmc::get(&ctx, &mm, autopsy_id).await?;
	assert_eq!(autopsy.meddra_version.as_deref(), Some("26.0"));

	let parent_c = ParentInformationForCreate {
		patient_id,
		sex: Some("2".to_string()),
		medical_history_text: None,
	};
	let parent_id = ParentInformationBmc::create(&ctx, &mm, parent_c).await?;
	let parent = ParentInformationBmc::get(&ctx, &mm, parent_id).await?;
	assert_eq!(parent.patient_id, patient_id);

	let parent_u = ParentInformationForUpdate {
		parent_identification: Some("Parent-1".to_string()),
		parent_birth_date: None,
		parent_age: Some(Decimal::new(30, 0)),
		parent_age_unit: Some("801".to_string()),
		last_menstrual_period_date: None,
		weight_kg: None,
		height_cm: None,
		sex: None,
		medical_history_text: None,
	};
	ParentInformationBmc::update(&ctx, &mm, parent_id, parent_u).await?;
	let parent = ParentInformationBmc::get(&ctx, &mm, parent_id).await?;
	assert_eq!(parent.parent_age, Some(Decimal::new(30, 0)));

	ParentInformationBmc::delete(&ctx, &mm, parent_id).await?;
	AutopsyCauseOfDeathBmc::delete(&ctx, &mm, autopsy_id).await?;
	ReportedCauseOfDeathBmc::delete(&ctx, &mm, reported_id).await?;
	PatientDeathInformationBmc::delete(&ctx, &mm, death_id).await?;
	PastDrugHistoryBmc::delete(&ctx, &mm, past_id).await?;
	MedicalHistoryEpisodeBmc::delete(&ctx, &mm, med_id).await?;
	PatientInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
