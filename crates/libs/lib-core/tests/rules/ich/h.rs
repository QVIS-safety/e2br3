use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_lacks_issue, create_case_with_safety_report,
	validate_case,
};
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationForCreate,
	CaseSummaryInformationForUpdate, NarrativeInformationBmc,
	NarrativeInformationForCreate, NarrativeInformationForUpdate,
	SenderDiagnosisBmc, SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
};
use lib_core::validation::ValidationProfile;
use serial_test::serial;
use sqlx::types::Uuid;

fn blank_narrative_update() -> NarrativeInformationForUpdate {
	NarrativeInformationForUpdate {
		case_narrative: None,
		reporter_comments: None,
		sender_comments: None,
	}
}

fn blank_sender_diagnosis_update() -> SenderDiagnosisForUpdate {
	SenderDiagnosisForUpdate {
		diagnosis_meddra_version: None,
		diagnosis_meddra_code: None,
	}
}

fn blank_case_summary_update() -> CaseSummaryInformationForUpdate {
	CaseSummaryInformationForUpdate {
		summary_type: None,
		language_code: None,
		summary_text: None,
	}
}

async fn create_narrative_case(
	case_narrative: &str,
) -> Result<(
	lib_core::ctx::Ctx,
	lib_core::model::ModelManager,
	Uuid,
	Uuid,
)> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let narrative_id = NarrativeInformationBmc::create(
		&ctx,
		&mm,
		NarrativeInformationForCreate {
			case_id,
			case_narrative: case_narrative.to_string(),
		},
	)
	.await?;

	Ok((ctx, mm, case_id, narrative_id))
}

#[serial]
#[tokio::test]
async fn ich_h_1_required_false() -> Result<()> {
	let (ctx, mm, case_id, _narrative_id) = create_narrative_case("").await?;
	let mut narrative_u = blank_narrative_update();
	narrative_u.reporter_comments = Some("Reporter comment".to_string());
	NarrativeInformationBmc::update_by_case(&ctx, &mm, case_id, narrative_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.H.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_h_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, _narrative_id) =
		create_narrative_case("Narrative").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.H.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_h_3_r_1a_required_false() -> Result<()> {
	let (ctx, mm, case_id, narrative_id) =
		create_narrative_case("Narrative").await?;
	SenderDiagnosisBmc::create(
		&ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id,
			sequence_number: 1,
			diagnosis_meddra_code: Some("10012345".to_string()),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.H.3.r.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_h_3_r_1a_required_true() -> Result<()> {
	let (ctx, mm, case_id, narrative_id) =
		create_narrative_case("Narrative").await?;
	let diagnosis_id = SenderDiagnosisBmc::create(
		&ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id,
			sequence_number: 1,
			diagnosis_meddra_code: Some("10012345".to_string()),
		},
	)
	.await?;
	let mut diagnosis_u = blank_sender_diagnosis_update();
	diagnosis_u.diagnosis_meddra_version = Some("27.0".to_string());
	SenderDiagnosisBmc::update(&ctx, &mm, diagnosis_id, diagnosis_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.H.3.r.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_h_3_r_1b_required_false() -> Result<()> {
	let (ctx, mm, case_id, narrative_id) =
		create_narrative_case("Narrative").await?;
	let diagnosis_id = SenderDiagnosisBmc::create(
		&ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id,
			sequence_number: 1,
			diagnosis_meddra_code: None,
		},
	)
	.await?;
	let mut diagnosis_u = blank_sender_diagnosis_update();
	diagnosis_u.diagnosis_meddra_version = Some("27.0".to_string());
	SenderDiagnosisBmc::update(&ctx, &mm, diagnosis_id, diagnosis_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.H.3.r.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_h_3_r_1b_required_true() -> Result<()> {
	let (ctx, mm, case_id, narrative_id) =
		create_narrative_case("Narrative").await?;
	let diagnosis_id = SenderDiagnosisBmc::create(
		&ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id,
			sequence_number: 1,
			diagnosis_meddra_code: Some("10012345".to_string()),
		},
	)
	.await?;
	let mut diagnosis_u = blank_sender_diagnosis_update();
	diagnosis_u.diagnosis_meddra_version = Some("27.0".to_string());
	SenderDiagnosisBmc::update(&ctx, &mm, diagnosis_id, diagnosis_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.H.3.r.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_h_5_r_1b_required_false() -> Result<()> {
	let (ctx, mm, case_id, narrative_id) =
		create_narrative_case("Narrative").await?;
	let summary_id = CaseSummaryInformationBmc::create(
		&ctx,
		&mm,
		CaseSummaryInformationForCreate {
			narrative_id,
			sequence_number: 1,
			summary_text: Some("Summary".to_string()),
		},
	)
	.await?;
	let mut summary_u = blank_case_summary_update();
	summary_u.summary_type = Some("01".to_string());
	CaseSummaryInformationBmc::update(&ctx, &mm, summary_id, summary_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.H.5.r.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_h_5_r_1b_required_true() -> Result<()> {
	let (ctx, mm, case_id, narrative_id) =
		create_narrative_case("Narrative").await?;
	let summary_id = CaseSummaryInformationBmc::create(
		&ctx,
		&mm,
		CaseSummaryInformationForCreate {
			narrative_id,
			sequence_number: 1,
			summary_text: Some("Summary".to_string()),
		},
	)
	.await?;
	let mut summary_u = blank_case_summary_update();
	summary_u.summary_type = Some("01".to_string());
	summary_u.language_code = Some("en".to_string());
	CaseSummaryInformationBmc::update(&ctx, &mm, summary_id, summary_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.H.5.r.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}
