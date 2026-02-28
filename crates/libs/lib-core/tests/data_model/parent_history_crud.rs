use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryForCreate,
	ParentMedicalHistoryForUpdate, ParentPastDrugHistoryBmc,
	ParentPastDrugHistoryForCreate, ParentPastDrugHistoryForUpdate,
};
use lib_core::model::patient::{
	ParentInformationBmc, ParentInformationForCreate, ParentInformationForUpdate,
	PatientInformationBmc, PatientInformationForCreate,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_parent_history_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let patient_c = PatientInformationForCreate {
		case_id,
		patient_initials: Some("PH".to_string()),
		sex: Some("2".to_string()),
		concomitant_therapy: None,
	};
	let patient_id = PatientInformationBmc::create(&ctx, &mm, patient_c).await?;

	let parent_c = ParentInformationForCreate {
		patient_id,
		sex: Some("2".to_string()),
		medical_history_text: Some("Family history baseline".to_string()),
	};
	let parent_id = ParentInformationBmc::create(&ctx, &mm, parent_c).await?;
	let parent = ParentInformationBmc::get(&ctx, &mm, parent_id).await?;
	assert_eq!(
		parent.medical_history_text.as_deref(),
		Some("Family history baseline")
	);

	let parent_u = ParentInformationForUpdate {
		parent_identification: None,
		parent_birth_date: None,
		parent_age: None,
		parent_age_unit: None,
		last_menstrual_period_date: None,
		weight_kg: None,
		height_cm: None,
		sex: None,
		medical_history_text: Some("Updated parent history".to_string()),
	};
	ParentInformationBmc::update(&ctx, &mm, parent_id, parent_u).await?;
	let parent = ParentInformationBmc::get(&ctx, &mm, parent_id).await?;
	assert_eq!(
		parent.medical_history_text.as_deref(),
		Some("Updated parent history")
	);

	let med_history_c = ParentMedicalHistoryForCreate {
		parent_id,
		sequence_number: 1,
		meddra_code: Some("10012345".to_string()),
	};
	let med_history_id =
		ParentMedicalHistoryBmc::create(&ctx, &mm, med_history_c).await?;
	let med_history =
		ParentMedicalHistoryBmc::get(&ctx, &mm, med_history_id).await?;
	assert_eq!(med_history.sequence_number, 1);

	let med_history_u = ParentMedicalHistoryForUpdate {
		meddra_version: Some("25.0".to_string()),
		meddra_code: None,
		start_date: None,
		continuing: None,
		end_date: None,
		comments: Some("Family history noted".to_string()),
	};
	ParentMedicalHistoryBmc::update(&ctx, &mm, med_history_id, med_history_u)
		.await?;
	let med_history =
		ParentMedicalHistoryBmc::get(&ctx, &mm, med_history_id).await?;
	assert_eq!(
		med_history.comments.as_deref(),
		Some("Family history noted")
	);

	let med_histories = ParentMedicalHistoryBmc::list(&ctx, &mm, None, None).await?;
	assert!(med_histories.iter().any(|m| m.id == med_history_id));

	let past_drug_c = ParentPastDrugHistoryForCreate {
		parent_id,
		sequence_number: 1,
		drug_name: Some("Legacy Drug".to_string()),
	};
	let past_drug_id =
		ParentPastDrugHistoryBmc::create(&ctx, &mm, past_drug_c).await?;
	let past_drug = ParentPastDrugHistoryBmc::get(&ctx, &mm, past_drug_id).await?;
	assert_eq!(past_drug.sequence_number, 1);

	let past_drug_u = ParentPastDrugHistoryForUpdate {
		drug_name: Some("Updated Drug".to_string()),
		mpid: Some("MP-123".to_string()),
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		start_date: None,
		end_date: None,
		indication_meddra_version: None,
		indication_meddra_code: Some("10078901".to_string()),
		reaction_meddra_version: None,
		reaction_meddra_code: None,
	};
	ParentPastDrugHistoryBmc::update(&ctx, &mm, past_drug_id, past_drug_u).await?;
	let past_drug = ParentPastDrugHistoryBmc::get(&ctx, &mm, past_drug_id).await?;
	assert_eq!(past_drug.drug_name.as_deref(), Some("Updated Drug"));

	let past_drugs = ParentPastDrugHistoryBmc::list(&ctx, &mm, None, None).await?;
	assert!(past_drugs.iter().any(|p| p.id == past_drug_id));

	ParentPastDrugHistoryBmc::delete(&ctx, &mm, past_drug_id).await?;
	ParentMedicalHistoryBmc::delete(&ctx, &mm, med_history_id).await?;
	ParentInformationBmc::delete(&ctx, &mm, parent_id).await?;
	PatientInformationBmc::delete(&ctx, &mm, patient_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}
