use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationForCreate,
	CaseSummaryInformationForUpdate, NarrativeInformationBmc,
	NarrativeInformationForCreate, NarrativeInformationForUpdate,
	SenderDiagnosisBmc, SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_narrative_information_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let narrative_c = NarrativeInformationForCreate {
		case_id,
		case_narrative: "Initial narrative".to_string(),
	};
	let narrative_id =
		NarrativeInformationBmc::create(&ctx, &mm, narrative_c).await?;
	let narrative = NarrativeInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(narrative.id, narrative_id);

	let narrative_u = NarrativeInformationForUpdate {
		case_narrative: Some("Updated narrative".to_string()),
		reporter_comments: None,
		sender_comments: None,
	};
	NarrativeInformationBmc::update_by_case(&ctx, &mm, case_id, narrative_u).await?;
	let narrative = NarrativeInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(narrative.case_narrative, "Updated narrative");

	NarrativeInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_submodels_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let narrative_c = NarrativeInformationForCreate {
		case_id,
		case_narrative: "Narrative for submodels".to_string(),
	};
	let narrative_id =
		NarrativeInformationBmc::create(&ctx, &mm, narrative_c).await?;

	let sender_diag_c = SenderDiagnosisForCreate {
		narrative_id,
		sequence_number: 1,
		diagnosis_meddra_code: Some("12345678".to_string()),
	};
	let sender_diag_id =
		SenderDiagnosisBmc::create(&ctx, &mm, sender_diag_c).await?;
	let sender_diag = SenderDiagnosisBmc::get(&ctx, &mm, sender_diag_id).await?;
	assert_eq!(sender_diag.sequence_number, 1);

	let sender_diag_u = SenderDiagnosisForUpdate {
		diagnosis_meddra_version: Some("26.0".to_string()),
		diagnosis_meddra_code: None,
	};
	SenderDiagnosisBmc::update(&ctx, &mm, sender_diag_id, sender_diag_u).await?;
	let sender_diag = SenderDiagnosisBmc::get(&ctx, &mm, sender_diag_id).await?;
	assert_eq!(
		sender_diag.diagnosis_meddra_version.as_deref(),
		Some("26.0")
	);

	let sender_diags = SenderDiagnosisBmc::list(&ctx, &mm, None, None).await?;
	assert!(sender_diags.iter().any(|d| d.id == sender_diag_id));

	SenderDiagnosisBmc::delete(&ctx, &mm, sender_diag_id).await?;

	let summary_c = CaseSummaryInformationForCreate {
		narrative_id,
		sequence_number: 1,
		summary_text: Some("Summary text".to_string()),
	};
	let summary_id = CaseSummaryInformationBmc::create(&ctx, &mm, summary_c).await?;
	let summary = CaseSummaryInformationBmc::get(&ctx, &mm, summary_id).await?;
	assert_eq!(summary.sequence_number, 1);

	let summary_u = CaseSummaryInformationForUpdate {
		summary_type: Some("01".to_string()),
		language_code: Some("en".to_string()),
		summary_text: None,
	};
	CaseSummaryInformationBmc::update(&ctx, &mm, summary_id, summary_u).await?;
	let summary = CaseSummaryInformationBmc::get(&ctx, &mm, summary_id).await?;
	assert_eq!(summary.summary_type.as_deref(), Some("01"));

	let summaries = CaseSummaryInformationBmc::list(&ctx, &mm, None, None).await?;
	assert!(summaries.iter().any(|s| s.id == summary_id));

	CaseSummaryInformationBmc::delete(&ctx, &mm, summary_id).await?;

	NarrativeInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
