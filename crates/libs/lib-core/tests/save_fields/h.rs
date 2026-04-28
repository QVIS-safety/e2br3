use super::common::{create_narrative, finish, setup_case};
use crate::test_common::Result;
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationForCreate,
	CaseSummaryInformationForUpdate, NarrativeInformationBmc,
	NarrativeInformationForCreate, NarrativeInformationForUpdate,
	SenderDiagnosisBmc, SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn save_h_1_2_4_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	create_narrative(&ctx, &mm, case_id).await?;
	let row = NarrativeInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.case_narrative, "Narrative");
	assert_eq!(row.reporter_comments, None);
	assert_eq!(row.sender_comments, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_h_1_2_4_create_full_surface() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	NarrativeInformationBmc::create(
		&ctx,
		&mm,
		NarrativeInformationForCreate {
			case_id,
			case_narrative: "Narrative 2".to_string(),
			reporter_comments: Some("Reporter".to_string()),
			sender_comments: Some("Sender".to_string()),
		},
	)
	.await?;
	let row = NarrativeInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.case_narrative, "Narrative 2");
	assert_eq!(row.reporter_comments.as_deref(), Some("Reporter"));
	assert_eq!(row.sender_comments.as_deref(), Some("Sender"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_h_1_2_4_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	create_narrative(&ctx, &mm, case_id).await?;
	NarrativeInformationBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		NarrativeInformationForUpdate {
			case_narrative: Some("Narrative 2".to_string()),
			reporter_comments: Some("Reporter".to_string()),
			sender_comments: Some("Sender".to_string()),
		},
	)
	.await?;
	let row = NarrativeInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.case_narrative, "Narrative 2");
	assert_eq!(row.reporter_comments.as_deref(), Some("Reporter"));
	assert_eq!(row.sender_comments.as_deref(), Some("Sender"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_h_3_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let narrative_id = create_narrative(&ctx, &mm, case_id).await?;
	let id = SenderDiagnosisBmc::create(
		&ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id,
			sequence_number: 1,
			diagnosis_meddra_code: Some("100".to_string()),
		},
	)
	.await?;
	let row = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.narrative_id, narrative_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.diagnosis_meddra_version, None);
	assert_eq!(row.diagnosis_meddra_code.as_deref(), Some("100"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_h_3_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let narrative_id = create_narrative(&ctx, &mm, case_id).await?;
	let id = SenderDiagnosisBmc::create(
		&ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id,
			sequence_number: 1,
			diagnosis_meddra_code: None,
		},
	)
	.await?;
	SenderDiagnosisBmc::update(
		&ctx,
		&mm,
		id,
		SenderDiagnosisForUpdate {
			diagnosis_meddra_version: Some("27.0".to_string()),
			diagnosis_meddra_code: Some("101".to_string()),
		},
	)
	.await?;
	let row = SenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.narrative_id, narrative_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.diagnosis_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.diagnosis_meddra_code.as_deref(), Some("101"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_h_5_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let narrative_id = create_narrative(&ctx, &mm, case_id).await?;
	let id = CaseSummaryInformationBmc::create(
		&ctx,
		&mm,
		CaseSummaryInformationForCreate {
			narrative_id,
			sequence_number: 1,
			summary_text: Some("Summary".to_string()),
		},
	)
	.await?;
	let row = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.narrative_id, narrative_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.summary_type, None);
	assert_eq!(row.language_code, None);
	assert_eq!(row.summary_text.as_deref(), Some("Summary"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_h_5_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let narrative_id = create_narrative(&ctx, &mm, case_id).await?;
	let id = CaseSummaryInformationBmc::create(
		&ctx,
		&mm,
		CaseSummaryInformationForCreate {
			narrative_id,
			sequence_number: 1,
			summary_text: None,
		},
	)
	.await?;
	CaseSummaryInformationBmc::update(
		&ctx,
		&mm,
		id,
		CaseSummaryInformationForUpdate {
			summary_type: Some("2".to_string()),
			language_code: Some("en".to_string()),
			summary_text: Some("Summary 2".to_string()),
		},
	)
	.await?;
	let row = CaseSummaryInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.narrative_id, narrative_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.summary_type.as_deref(), Some("2"));
	assert_eq!(row.language_code.as_deref(), Some("en"));
	assert_eq!(row.summary_text.as_deref(), Some("Summary 2"));
	finish(&mm).await
}
