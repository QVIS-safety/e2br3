use super::common::{create_patient, date, dec, finish, setup_case};
use crate::test_common::Result;
use lib_core::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryForCreate,
	ParentMedicalHistoryForUpdate, ParentPastDrugHistoryBmc,
	ParentPastDrugHistoryForCreate, ParentPastDrugHistoryForUpdate,
};
use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathForCreate,
	AutopsyCauseOfDeathForUpdate, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeForCreate, MedicalHistoryEpisodeForUpdate,
	ParentInformationBmc, ParentInformationForCreate, ParentInformationForUpdate,
	PastDrugHistoryBmc, PastDrugHistoryForCreate, PastDrugHistoryForUpdate,
	PatientDeathInformationBmc, PatientDeathInformationForCreate,
	PatientDeathInformationForUpdate, PatientIdentifierBmc,
	PatientIdentifierForCreate, PatientIdentifierForUpdate, PatientInformationBmc,
	PatientInformationForUpdate, ReportedCauseOfDeathBmc,
	ReportedCauseOfDeathForCreate, ReportedCauseOfDeathForUpdate,
};
use serial_test::serial;
use time::Month;

#[tokio::test]
#[serial]
async fn save_d_1_2_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = create_patient(&ctx, &mm, case_id).await?;
	let row = PatientInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.patient_initials.as_deref(), Some("PT"));
	assert_eq!(row.patient_given_name, None);
	assert_eq!(row.patient_family_name, None);
	assert_eq!(row.birth_date, None);
	assert_eq!(row.age_at_time_of_onset, None);
	assert_eq!(row.age_unit, None);
	assert_eq!(row.gestation_period, None);
	assert_eq!(row.gestation_period_unit, None);
	assert_eq!(row.age_group, None);
	assert_eq!(row.weight_kg, None);
	assert_eq!(row.height_cm, None);
	assert_eq!(row.sex.as_deref(), Some("1"));
	assert_eq!(row.patient_initials_null_flavor, None);
	assert_eq!(row.birth_date_null_flavor, None);
	assert_eq!(row.age_at_time_of_onset_null_flavor, None);
	assert_eq!(row.sex_null_flavor, None);
	assert_eq!(row.race_code, None);
	assert_eq!(row.ethnicity_code, None);
	assert_eq!(row.last_menstrual_period_date, None);
	assert_eq!(row.last_menstrual_period_date_null_flavor, None);
	assert_eq!(row.medical_history_text, None);
	assert_eq!(row.concomitant_therapy, Some(false));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_1_2_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	create_patient(&ctx, &mm, case_id).await?;
	PatientInformationBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		PatientInformationForUpdate {
			patient_initials: Some("AB".to_string()),
			patient_given_name: Some("Alice".to_string()),
			patient_family_name: Some("Brown".to_string()),
			patient_initials_null_flavor: None,
			birth_date: Some(date(2020, Month::January, 1)),
			birth_date_null_flavor: None,
			age_at_time_of_onset: Some(dec(33, 0)),
			age_at_time_of_onset_null_flavor: None,
			age_unit: Some("801".to_string()),
			gestation_period: Some(dec(10, 0)),
			gestation_period_unit: Some("804".to_string()),
			age_group: Some("1".to_string()),
			weight_kg: Some(dec(7000, 2)),
			height_cm: Some(dec(17500, 2)),
			sex: Some("2".to_string()),
			sex_null_flavor: None,
			race_code: Some("R1".to_string()),
			ethnicity_code: Some("E1".to_string()),
			last_menstrual_period_date: Some(date(2023, Month::December, 1)),
			last_menstrual_period_date_null_flavor: None,
			medical_history_text: Some("History".to_string()),
			concomitant_therapy: Some(true),
		},
	)
	.await?;
	let row = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.patient_initials.as_deref(), Some("AB"));
	assert_eq!(row.patient_given_name.as_deref(), Some("Alice"));
	assert_eq!(row.patient_family_name.as_deref(), Some("Brown"));
	assert_eq!(row.birth_date, Some(date(2020, Month::January, 1)));
	assert_eq!(row.age_at_time_of_onset, Some(dec(33, 0)));
	assert_eq!(row.age_unit.as_deref(), Some("801"));
	assert_eq!(row.gestation_period, Some(dec(10, 0)));
	assert_eq!(row.gestation_period_unit.as_deref(), Some("804"));
	assert_eq!(row.age_group.as_deref(), Some("1"));
	assert_eq!(row.weight_kg, Some(dec(7000, 2)));
	assert_eq!(row.height_cm, Some(dec(17500, 2)));
	assert_eq!(row.sex.as_deref(), Some("2"));
	assert_eq!(row.patient_initials_null_flavor, None);
	assert_eq!(row.birth_date_null_flavor, None);
	assert_eq!(row.age_at_time_of_onset_null_flavor, None);
	assert_eq!(row.sex_null_flavor, None);
	assert_eq!(row.race_code.as_deref(), Some("R1"));
	assert_eq!(row.ethnicity_code.as_deref(), Some("E1"));
	assert_eq!(
		row.last_menstrual_period_date,
		Some(date(2023, Month::December, 1))
	);
	assert_eq!(row.last_menstrual_period_date_null_flavor, None);
	assert_eq!(row.medical_history_text.as_deref(), Some("History"));
	assert_eq!(row.concomitant_therapy, Some(true));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_2_1_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = PatientIdentifierBmc::create(
		&ctx,
		&mm,
		PatientIdentifierForCreate {
			patient_id,
			sequence_number: 1,
			identifier_type_code: "1".to_string(),
			identifier_value: "123".to_string(),
		},
	)
	.await?;
	let row = PatientIdentifierBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.identifier_type_code, "1");
	assert_eq!(row.identifier_value, "123");
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_2_1_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = PatientIdentifierBmc::create(
		&ctx,
		&mm,
		PatientIdentifierForCreate {
			patient_id,
			sequence_number: 1,
			identifier_type_code: "1".to_string(),
			identifier_value: "123".to_string(),
		},
	)
	.await?;
	PatientIdentifierBmc::update(
		&ctx,
		&mm,
		id,
		PatientIdentifierForUpdate {
			identifier_type_code: Some("2".to_string()),
			identifier_value: Some("456".to_string()),
		},
	)
	.await?;
	let row = PatientIdentifierBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.identifier_type_code, "2");
	assert_eq!(row.identifier_value, "456");
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_7_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = MedicalHistoryEpisodeBmc::create(
		&ctx,
		&mm,
		MedicalHistoryEpisodeForCreate {
			patient_id,
			sequence_number: 1,
			meddra_code: Some("100".to_string()),
			start_date_null_flavor: Some("NI".to_string()),
			end_date_null_flavor: Some("UNK".to_string()),
		},
	)
	.await?;
	let row = MedicalHistoryEpisodeBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.meddra_version, None);
	assert_eq!(row.meddra_code.as_deref(), Some("100"));
	assert_eq!(row.start_date, None);
	assert_eq!(row.start_date_null_flavor.as_deref(), Some("NI"));
	assert_eq!(row.continuing, None);
	assert_eq!(row.end_date, None);
	assert_eq!(row.end_date_null_flavor.as_deref(), Some("UNK"));
	assert_eq!(row.comments, None);
	assert_eq!(row.family_history, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_7_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = MedicalHistoryEpisodeBmc::create(
		&ctx,
		&mm,
		MedicalHistoryEpisodeForCreate {
			patient_id,
			sequence_number: 1,
			meddra_code: None,
			start_date_null_flavor: None,
			end_date_null_flavor: None,
		},
	)
	.await?;
	MedicalHistoryEpisodeBmc::update(
		&ctx,
		&mm,
		id,
		MedicalHistoryEpisodeForUpdate {
			meddra_version: Some("27.0".to_string()),
			meddra_code: Some("200".to_string()),
			start_date: Some(date(2024, Month::January, 1)),
			start_date_null_flavor: None,
			continuing: Some(true),
			end_date: Some(date(2024, Month::January, 2)),
			end_date_null_flavor: None,
			comments: Some("Comment".to_string()),
			family_history: Some(false),
		},
	)
	.await?;
	let row = MedicalHistoryEpisodeBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.meddra_code.as_deref(), Some("200"));
	assert_eq!(row.start_date, Some(date(2024, Month::January, 1)));
	assert_eq!(row.start_date_null_flavor, None);
	assert_eq!(row.continuing, Some(true));
	assert_eq!(row.end_date, Some(date(2024, Month::January, 2)));
	assert_eq!(row.end_date_null_flavor, None);
	assert_eq!(row.comments.as_deref(), Some("Comment"));
	assert_eq!(row.family_history, Some(false));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_8_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = PastDrugHistoryBmc::create(
		&ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id,
			sequence_number: 1,
			drug_name: Some("Drug".to_string()),
			drug_name_null_flavor: None,
			mpid: Some("MPID".to_string()),
			mpid_version: Some("1".to_string()),
			phpid: Some("PHPID".to_string()),
			phpid_version: Some("2".to_string()),
			start_date: Some(date(2024, Month::January, 1)),
			start_date_null_flavor: None,
			end_date: Some(date(2024, Month::January, 2)),
			end_date_null_flavor: None,
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: Some("300".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			reaction_meddra_code: Some("400".to_string()),
		},
	)
	.await?;
	let row = PastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.drug_name.as_deref(), Some("Drug"));
	assert_eq!(row.drug_name_null_flavor, None);
	assert_eq!(row.mpid.as_deref(), Some("MPID"));
	assert_eq!(row.mpid_version.as_deref(), Some("1"));
	assert_eq!(row.phpid.as_deref(), Some("PHPID"));
	assert_eq!(row.phpid_version.as_deref(), Some("2"));
	assert_eq!(row.start_date, Some(date(2024, Month::January, 1)));
	assert_eq!(row.start_date_null_flavor, None);
	assert_eq!(row.end_date, Some(date(2024, Month::January, 2)));
	assert_eq!(row.end_date_null_flavor, None);
	assert_eq!(row.indication_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.indication_meddra_code.as_deref(), Some("300"));
	assert_eq!(row.reaction_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.reaction_meddra_code.as_deref(), Some("400"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_8_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = PastDrugHistoryBmc::create(
		&ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id,
			sequence_number: 1,
			drug_name: None,
			drug_name_null_flavor: Some("NI".to_string()),
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: Some("UNK".to_string()),
			end_date: None,
			end_date_null_flavor: Some("ASKU".to_string()),
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;
	PastDrugHistoryBmc::update(
		&ctx,
		&mm,
		id,
		PastDrugHistoryForUpdate {
			drug_name: Some("Drug 2".to_string()),
			drug_name_null_flavor: None,
			mpid: Some("MPID2".to_string()),
			mpid_version: Some("2".to_string()),
			phpid: Some("PHPID2".to_string()),
			phpid_version: Some("3".to_string()),
			start_date: Some(date(2024, Month::February, 1)),
			start_date_null_flavor: None,
			end_date: Some(date(2024, Month::February, 2)),
			end_date_null_flavor: None,
			indication_meddra_version: Some("28.0".to_string()),
			indication_meddra_code: Some("301".to_string()),
			reaction_meddra_version: Some("28.0".to_string()),
			reaction_meddra_code: Some("401".to_string()),
		},
	)
	.await?;
	let row = PastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.drug_name.as_deref(), Some("Drug 2"));
	assert_eq!(row.drug_name_null_flavor, None);
	assert_eq!(row.mpid.as_deref(), Some("MPID2"));
	assert_eq!(row.mpid_version.as_deref(), Some("2"));
	assert_eq!(row.phpid.as_deref(), Some("PHPID2"));
	assert_eq!(row.phpid_version.as_deref(), Some("3"));
	assert_eq!(row.start_date, Some(date(2024, Month::February, 1)));
	assert_eq!(row.start_date_null_flavor, None);
	assert_eq!(row.end_date, Some(date(2024, Month::February, 2)));
	assert_eq!(row.end_date_null_flavor, None);
	assert_eq!(row.indication_meddra_version.as_deref(), Some("28.0"));
	assert_eq!(row.indication_meddra_code.as_deref(), Some("301"));
	assert_eq!(row.reaction_meddra_version.as_deref(), Some("28.0"));
	assert_eq!(row.reaction_meddra_code.as_deref(), Some("401"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_9_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id,
			date_of_death: Some(date(2024, Month::January, 10)),
			date_of_death_null_flavor: None,
			autopsy_performed: Some(true),
		},
	)
	.await?;
	let row = PatientDeathInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.date_of_death, Some(date(2024, Month::January, 10)));
	assert_eq!(row.date_of_death_null_flavor, None);
	assert_eq!(row.autopsy_performed, Some(true));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_9_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id,
			date_of_death: None,
			date_of_death_null_flavor: Some("UNK".to_string()),
			autopsy_performed: Some(false),
		},
	)
	.await?;
	PatientDeathInformationBmc::update(
		&ctx,
		&mm,
		id,
		PatientDeathInformationForUpdate {
			date_of_death: Some(date(2024, Month::February, 10)),
			date_of_death_null_flavor: None,
			autopsy_performed: Some(true),
		},
	)
	.await?;
	let row = PatientDeathInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.date_of_death, Some(date(2024, Month::February, 10)));
	assert_eq!(row.date_of_death_null_flavor, None);
	assert_eq!(row.autopsy_performed, Some(true));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_9_1_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let death_id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id,
			date_of_death: None,
			date_of_death_null_flavor: None,
			autopsy_performed: Some(false),
		},
	)
	.await?;
	let id = ReportedCauseOfDeathBmc::create(
		&ctx,
		&mm,
		ReportedCauseOfDeathForCreate {
			death_info_id: death_id,
			sequence_number: 1,
			meddra_code: Some("500".to_string()),
		},
	)
	.await?;
	let row = ReportedCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.death_info_id, death_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.meddra_version, None);
	assert_eq!(row.meddra_code.as_deref(), Some("500"));
	assert_eq!(row.comments, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_9_1_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let death_id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id,
			date_of_death: None,
			date_of_death_null_flavor: None,
			autopsy_performed: Some(false),
		},
	)
	.await?;
	let id = ReportedCauseOfDeathBmc::create(
		&ctx,
		&mm,
		ReportedCauseOfDeathForCreate {
			death_info_id: death_id,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;
	ReportedCauseOfDeathBmc::update(
		&ctx,
		&mm,
		id,
		ReportedCauseOfDeathForUpdate {
			meddra_version: Some("27.0".to_string()),
			meddra_code: Some("501".to_string()),
			comments: Some("Comment".to_string()),
		},
	)
	.await?;
	let row = ReportedCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.death_info_id, death_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.meddra_code.as_deref(), Some("501"));
	assert_eq!(row.comments.as_deref(), Some("Comment"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_9_2_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let death_id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id,
			date_of_death: None,
			date_of_death_null_flavor: None,
			autopsy_performed: Some(false),
		},
	)
	.await?;
	let id = AutopsyCauseOfDeathBmc::create(
		&ctx,
		&mm,
		AutopsyCauseOfDeathForCreate {
			death_info_id: death_id,
			sequence_number: 1,
			meddra_code: Some("600".to_string()),
		},
	)
	.await?;
	let row = AutopsyCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.death_info_id, death_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.meddra_version, None);
	assert_eq!(row.meddra_code.as_deref(), Some("600"));
	assert_eq!(row.comments, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_9_2_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let death_id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id,
			date_of_death: None,
			date_of_death_null_flavor: None,
			autopsy_performed: Some(false),
		},
	)
	.await?;
	let id = AutopsyCauseOfDeathBmc::create(
		&ctx,
		&mm,
		AutopsyCauseOfDeathForCreate {
			death_info_id: death_id,
			sequence_number: 1,
			meddra_code: None,
		},
	)
	.await?;
	AutopsyCauseOfDeathBmc::update(
		&ctx,
		&mm,
		id,
		AutopsyCauseOfDeathForUpdate {
			meddra_version: Some("27.0".to_string()),
			meddra_code: Some("601".to_string()),
			comments: Some("Comment".to_string()),
		},
	)
	.await?;
	let row = AutopsyCauseOfDeathBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.death_info_id, death_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.meddra_code.as_deref(), Some("601"));
	assert_eq!(row.comments.as_deref(), Some("Comment"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_10_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: Some("2".to_string()),
			medical_history_text: Some("History".to_string()),
		},
	)
	.await?;
	let row = ParentInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.parent_identification, None);
	assert_eq!(row.parent_birth_date, None);
	assert_eq!(row.parent_birth_date_null_flavor, None);
	assert_eq!(row.parent_age, None);
	assert_eq!(row.parent_age_null_flavor, None);
	assert_eq!(row.parent_age_unit, None);
	assert_eq!(row.last_menstrual_period_date, None);
	assert_eq!(row.last_menstrual_period_date_null_flavor, None);
	assert_eq!(row.weight_kg, None);
	assert_eq!(row.height_cm, None);
	assert_eq!(row.sex.as_deref(), Some("2"));
	assert_eq!(row.medical_history_text.as_deref(), Some("History"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_10_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: None,
			medical_history_text: None,
		},
	)
	.await?;
	ParentInformationBmc::update(
		&ctx,
		&mm,
		id,
		ParentInformationForUpdate {
			parent_identification: Some("PARENT-1".to_string()),
			parent_birth_date: Some(date(1980, Month::January, 1)),
			parent_birth_date_null_flavor: None,
			parent_age: Some(dec(44, 0)),
			parent_age_null_flavor: None,
			parent_age_unit: Some("801".to_string()),
			last_menstrual_period_date: Some(date(2023, Month::December, 1)),
			last_menstrual_period_date_null_flavor: None,
			weight_kg: Some(dec(6500, 2)),
			height_cm: Some(dec(16500, 2)),
			sex: Some("1".to_string()),
			medical_history_text: Some("Parent history".to_string()),
		},
	)
	.await?;
	let row = ParentInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.patient_id, patient_id);
	assert_eq!(row.parent_identification.as_deref(), Some("PARENT-1"));
	assert_eq!(row.parent_birth_date, Some(date(1980, Month::January, 1)));
	assert_eq!(row.parent_birth_date_null_flavor, None);
	assert_eq!(row.parent_age, Some(dec(44, 0)));
	assert_eq!(row.parent_age_null_flavor, None);
	assert_eq!(row.parent_age_unit.as_deref(), Some("801"));
	assert_eq!(
		row.last_menstrual_period_date,
		Some(date(2023, Month::December, 1))
	);
	assert_eq!(row.last_menstrual_period_date_null_flavor, None);
	assert_eq!(row.weight_kg, Some(dec(6500, 2)));
	assert_eq!(row.height_cm, Some(dec(16500, 2)));
	assert_eq!(row.sex.as_deref(), Some("1"));
	assert_eq!(row.medical_history_text.as_deref(), Some("Parent history"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_10_6_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let parent_id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: None,
			medical_history_text: None,
		},
	)
	.await?;
	let id = ParentMedicalHistoryBmc::create(
		&ctx,
		&mm,
		ParentMedicalHistoryForCreate {
			parent_id,
			sequence_number: 1,
			meddra_code: Some("700".to_string()),
			start_date_null_flavor: Some("NI".to_string()),
			end_date_null_flavor: Some("UNK".to_string()),
		},
	)
	.await?;
	let row = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.parent_id, parent_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.meddra_version, None);
	assert_eq!(row.meddra_code.as_deref(), Some("700"));
	assert_eq!(row.start_date, None);
	assert_eq!(row.start_date_null_flavor.as_deref(), Some("NI"));
	assert_eq!(row.continuing, None);
	assert_eq!(row.end_date, None);
	assert_eq!(row.end_date_null_flavor.as_deref(), Some("UNK"));
	assert_eq!(row.comments, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_10_6_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let parent_id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: None,
			medical_history_text: None,
		},
	)
	.await?;
	let id = ParentMedicalHistoryBmc::create(
		&ctx,
		&mm,
		ParentMedicalHistoryForCreate {
			parent_id,
			sequence_number: 1,
			meddra_code: None,
			start_date_null_flavor: None,
			end_date_null_flavor: None,
		},
	)
	.await?;
	ParentMedicalHistoryBmc::update(
		&ctx,
		&mm,
		id,
		ParentMedicalHistoryForUpdate {
			meddra_version: Some("27.0".to_string()),
			meddra_code: Some("701".to_string()),
			start_date: Some(date(2024, Month::March, 1)),
			start_date_null_flavor: None,
			continuing: Some(false),
			end_date: Some(date(2024, Month::March, 2)),
			end_date_null_flavor: None,
			comments: Some("Comment".to_string()),
		},
	)
	.await?;
	let row = ParentMedicalHistoryBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.parent_id, parent_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.meddra_code.as_deref(), Some("701"));
	assert_eq!(row.start_date, Some(date(2024, Month::March, 1)));
	assert_eq!(row.start_date_null_flavor, None);
	assert_eq!(row.continuing, Some(false));
	assert_eq!(row.end_date, Some(date(2024, Month::March, 2)));
	assert_eq!(row.end_date_null_flavor, None);
	assert_eq!(row.comments.as_deref(), Some("Comment"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_10_7_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let parent_id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: None,
			medical_history_text: None,
		},
	)
	.await?;
	let id = ParentPastDrugHistoryBmc::create(
		&ctx,
		&mm,
		ParentPastDrugHistoryForCreate {
			parent_id,
			sequence_number: 1,
			drug_name: Some("Drug".to_string()),
			drug_name_null_flavor: None,
			start_date_null_flavor: Some("NI".to_string()),
			end_date_null_flavor: Some("UNK".to_string()),
		},
	)
	.await?;
	let row = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.parent_id, parent_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.drug_name.as_deref(), Some("Drug"));
	assert_eq!(row.drug_name_null_flavor, None);
	assert_eq!(row.mpid, None);
	assert_eq!(row.mpid_version, None);
	assert_eq!(row.phpid, None);
	assert_eq!(row.phpid_version, None);
	assert_eq!(row.start_date, None);
	assert_eq!(row.start_date_null_flavor.as_deref(), Some("NI"));
	assert_eq!(row.end_date, None);
	assert_eq!(row.end_date_null_flavor.as_deref(), Some("UNK"));
	assert_eq!(row.indication_meddra_version, None);
	assert_eq!(row.indication_meddra_code, None);
	assert_eq!(row.reaction_meddra_version, None);
	assert_eq!(row.reaction_meddra_code, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_d_10_7_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let patient_id = create_patient(&ctx, &mm, case_id).await?;
	let parent_id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: None,
			medical_history_text: None,
		},
	)
	.await?;
	let id = ParentPastDrugHistoryBmc::create(
		&ctx,
		&mm,
		ParentPastDrugHistoryForCreate {
			parent_id,
			sequence_number: 1,
			drug_name: None,
			drug_name_null_flavor: Some("NI".to_string()),
			start_date_null_flavor: Some("UNK".to_string()),
			end_date_null_flavor: Some("ASKU".to_string()),
		},
	)
	.await?;
	ParentPastDrugHistoryBmc::update(
		&ctx,
		&mm,
		id,
		ParentPastDrugHistoryForUpdate {
			drug_name: Some("Drug 2".to_string()),
			drug_name_null_flavor: None,
			mpid: Some("MPID".to_string()),
			mpid_version: Some("1".to_string()),
			phpid: Some("PHPID".to_string()),
			phpid_version: Some("2".to_string()),
			start_date: Some(date(2024, Month::April, 1)),
			start_date_null_flavor: None,
			end_date: Some(date(2024, Month::April, 2)),
			end_date_null_flavor: None,
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: Some("800".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			reaction_meddra_code: Some("801".to_string()),
		},
	)
	.await?;
	let row = ParentPastDrugHistoryBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.parent_id, parent_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.drug_name.as_deref(), Some("Drug 2"));
	assert_eq!(row.drug_name_null_flavor, None);
	assert_eq!(row.mpid.as_deref(), Some("MPID"));
	assert_eq!(row.mpid_version.as_deref(), Some("1"));
	assert_eq!(row.phpid.as_deref(), Some("PHPID"));
	assert_eq!(row.phpid_version.as_deref(), Some("2"));
	assert_eq!(row.start_date, Some(date(2024, Month::April, 1)));
	assert_eq!(row.start_date_null_flavor, None);
	assert_eq!(row.end_date, Some(date(2024, Month::April, 2)));
	assert_eq!(row.end_date_null_flavor, None);
	assert_eq!(row.indication_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.indication_meddra_code.as_deref(), Some("800"));
	assert_eq!(row.reaction_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.reaction_meddra_code.as_deref(), Some("801"));
	finish(&mm).await
}
