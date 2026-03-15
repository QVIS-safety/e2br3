use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_lacks_issue, blank_safety_report_update,
	create_case_with_safety_report, update_safety_report, validate_case,
};
use lib_core::model::drug::{
	DrugActiveSubstanceBmc, DrugActiveSubstanceForCreate, DrugInformationBmc,
	DrugInformationForCreate, DrugInformationForUpdate,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	RelatednessAssessmentBmc, RelatednessAssessmentForCreate,
};
use lib_core::model::message_header::{MessageHeaderBmc, MessageHeaderForCreate};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::xml::validate::ValidationProfile;
use serial_test::serial;
use sqlx::types::Uuid;

fn blank_drug_update() -> DrugInformationForUpdate {
	DrugInformationForUpdate {
		medicinal_product: None,
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
		action_taken: None,
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
	}
}

async fn create_mfds_drug_case(
	receiver: &str,
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
	let message_number = format!("MSG-{}", Uuid::new_v4());
	MessageHeaderBmc::create(
		&ctx,
		&mm,
		MessageHeaderForCreate {
			case_id,
			message_number,
			message_sender_identifier: "SENDER".to_string(),
			message_receiver_identifier: receiver.to_string(),
			message_date: "20260313000000".to_string(),
		},
	)
	.await?;
	let drug_id = DrugInformationBmc::create(
		&ctx,
		&mm,
		DrugInformationForCreate {
			case_id,
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "MFDS Drug".to_string(),
		},
	)
	.await?;

	Ok((ctx, mm, case_id, drug_id))
}

#[serial]
#[tokio::test]
async fn mfds_g_k_2_1_kr_1a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("FR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.mpid = Some("MPID-001".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.G.k.2.1.KR.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_2_1_kr_1a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("FR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.mpid = Some("MPID-001".to_string());
	drug_u.mpid_version = Some("2024Q4".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.G.k.2.1.KR.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_2_1_kr_1b_required_false() -> Result<()> {
	let (ctx, mm, case_id, _drug_id) = create_mfds_drug_case("KR").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.G.k.2.1.KR.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_2_1_kr_1b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.mpid = Some("MPID-001".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.G.k.2.1.KR.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_2_3_r_1_kr_1a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("FR").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: Some("TERM-001".to_string()),
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_2_3_r_1_kr_1a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("FR").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: Some("TERM-001".to_string()),
			substance_termid_version: Some("27.0".to_string()),
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_2_3_r_1_kr_1b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_2_3_r_1_kr_1b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: Some("TERM-001".to_string()),
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_9_i_2_r_1_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: None,
			method_of_assessment: Some("1".to_string()),
			result_of_assessment: None,
			result_of_assessment_kr2: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.G.k.9.i.2.r.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_9_i_2_r_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: Some("1".to_string()),
			result_of_assessment: None,
			result_of_assessment_kr2: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.G.k.9.i.2.r.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_9_i_2_r_2_kr_1_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: None,
			result_of_assessment: None,
			result_of_assessment_kr2: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_9_i_2_r_2_kr_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: Some("1".to_string()),
			result_of_assessment: None,
			result_of_assessment_kr2: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_9_i_2_r_3_kr_1_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: Some("1".to_string()),
			result_of_assessment: None,
			result_of_assessment_kr2: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_9_i_2_r_3_kr_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: Some("1".to_string()),
			result_of_assessment: Some("2".to_string()),
			result_of_assessment_kr2: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_9_i_2_r_3_kr_2_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("CT").await?;
	let mut report_u = blank_safety_report_update();
	report_u.report_type = Some("2".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: Some("2".to_string()),
			result_of_assessment: None,
			result_of_assessment_kr2: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_g_k_9_i_2_r_3_kr_2_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("CT").await?;
	let mut report_u = blank_safety_report_update();
	report_u.report_type = Some("2".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: Some("2".to_string()),
			result_of_assessment: None,
			result_of_assessment_kr2: Some("KRCT".to_string()),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_kr_domestic_ingredientcode_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.obtain_drug_country = Some("KR".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_kr_domestic_ingredientcode_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.obtain_drug_country = Some("KR".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: Some("TERM-001".to_string()),
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_kr_domestic_productcode_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.obtain_drug_country = Some("KR".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_kr_domestic_productcode_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.obtain_drug_country = Some("KR".to_string());
	drug_u.mpid = Some("MPID-001".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_kr_foreign_whompid_recommended_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.obtain_drug_country = Some("US".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_has_issue(&report, "MFDS.KR.FOREIGN.WHOMPID.RECOMMENDED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn mfds_kr_foreign_whompid_recommended_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_mfds_drug_case("KR").await?;
	let mut drug_u = blank_drug_update();
	drug_u.obtain_drug_country = Some("US".to_string());
	drug_u.mpid = Some("MPID-001".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Mfds).await?;

	assert_lacks_issue(&report, "MFDS.KR.FOREIGN.WHOMPID.RECOMMENDED");
	commit_test_ctx(&mm).await?;
	Ok(())
}
