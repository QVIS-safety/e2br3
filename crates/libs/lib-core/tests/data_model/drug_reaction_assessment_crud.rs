use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{DrugInformationBmc, DrugInformationForCreate};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	DrugReactionAssessmentForUpdate, RelatednessAssessmentBmc,
	RelatednessAssessmentFilter, RelatednessAssessmentForCreate,
	RelatednessAssessmentForUpdate,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use modql::filter::{OpValValue, OpValsValue};
use rust_decimal::Decimal;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_drug_reaction_assessment_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Assessment Drug".to_string(),
		drug_generic_name: None,
		..Default::default()
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	let reaction_c = ReactionForCreate {
		case_id,
		sequence_number: 1,
		primary_source_reaction: "Headache".to_string(),
	};
	let reaction_id = ReactionBmc::create(&ctx, &mm, reaction_c).await?;

	let assessment_c = DrugReactionAssessmentForCreate {
		drug_id,
		reaction_id,
	};
	let assessment_id =
		DrugReactionAssessmentBmc::create(&ctx, &mm, assessment_c).await?;
	let assessment =
		DrugReactionAssessmentBmc::get(&ctx, &mm, assessment_id).await?;
	assert_eq!(assessment.drug_id, drug_id);

	let assessment_u = DrugReactionAssessmentForUpdate {
		administration_start_interval_value: Some(Decimal::new(12, 0)),
		administration_start_interval_unit: Some("805".to_string()),
		last_dose_interval_value: Some(Decimal::new(3, 0)),
		last_dose_interval_unit: Some("804".to_string()),
		recurrence_action: Some("1".to_string()),
		recurrence_meddra_version: Some("25.0".to_string()),
		recurrence_meddra_code: Some("10012345".to_string()),
		reaction_recurred: Some("1".to_string()),
	};
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;
	let assessment =
		DrugReactionAssessmentBmc::get(&ctx, &mm, assessment_id).await?;
	assert_eq!(
		assessment.administration_start_interval_unit.as_deref(),
		Some("805")
	);
	assert_eq!(assessment.last_dose_interval_unit.as_deref(), Some("804"));
	assert_eq!(assessment.reaction_recurred.as_deref(), Some("1"));

	let by_drug =
		DrugReactionAssessmentBmc::list_by_drug(&ctx, &mm, drug_id).await?;
	assert!(by_drug.iter().any(|a| a.id == assessment_id));
	let by_reaction =
		DrugReactionAssessmentBmc::list_by_reaction(&ctx, &mm, reaction_id).await?;
	assert!(by_reaction.iter().any(|a| a.id == assessment_id));
	let by_pair = DrugReactionAssessmentBmc::get_by_drug_and_reaction(
		&ctx,
		&mm,
		drug_id,
		reaction_id,
	)
	.await?;
	assert!(by_pair.is_some());

	let related_c = RelatednessAssessmentForCreate {
		drug_reaction_assessment_id: assessment_id,
		sequence_number: 1,
		source_of_assessment: None,
		method_of_assessment: None,
		result_of_assessment: None,
		result_of_assessment_kr2: None,
	};
	let related_id = RelatednessAssessmentBmc::create(&ctx, &mm, related_c).await?;
	let related = RelatednessAssessmentBmc::get(&ctx, &mm, related_id).await?;
	assert_eq!(related.sequence_number, 1);

	let related_u = RelatednessAssessmentForUpdate {
		source_of_assessment: Some("Reporter".to_string()),
		method_of_assessment: Some("Expert judgement".to_string()),
		result_of_assessment: Some("Related".to_string()),
		result_of_assessment_kr2: None,
	};
	RelatednessAssessmentBmc::update(&ctx, &mm, related_id, related_u).await?;
	let related = RelatednessAssessmentBmc::get(&ctx, &mm, related_id).await?;
	assert_eq!(related.source_of_assessment.as_deref(), Some("Reporter"));

	let related_list = RelatednessAssessmentBmc::list(
		&ctx,
		&mm,
		Some(vec![RelatednessAssessmentFilter {
			drug_reaction_assessment_id: Some(OpValsValue::from(vec![
				OpValValue::Eq(assessment_id.to_string().into()),
			])),
			sequence_number: None,
		}]),
		None,
	)
	.await?;
	assert!(related_list.iter().any(|r| r.id == related_id));

	RelatednessAssessmentBmc::delete(&ctx, &mm, related_id).await?;
	DrugReactionAssessmentBmc::delete(&ctx, &mm, assessment_id).await?;
	ReactionBmc::delete(&ctx, &mm, reaction_id).await?;
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}
