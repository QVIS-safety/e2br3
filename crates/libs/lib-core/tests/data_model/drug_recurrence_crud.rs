use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{DrugInformationBmc, DrugInformationForCreate};
use lib_core::model::drug_recurrence::{
	DrugRecurrenceInformationBmc, DrugRecurrenceInformationForCreate,
	DrugRecurrenceInformationForUpdate,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_drug_recurrence_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Recurrence Drug".to_string(),
		drug_generic_name: None,
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	let recurrence_c = DrugRecurrenceInformationForCreate {
		drug_id,
		sequence_number: 1,
	};
	let recurrence_id =
		DrugRecurrenceInformationBmc::create(&ctx, &mm, recurrence_c).await?;
	let recurrence =
		DrugRecurrenceInformationBmc::get(&ctx, &mm, recurrence_id).await?;
	assert_eq!(recurrence.sequence_number, 1);

	let recurrence_u = DrugRecurrenceInformationForUpdate {
		rechallenge_action: Some("1".to_string()),
		reaction_meddra_version: None,
		reaction_meddra_code: Some("12345678".to_string()),
		reaction_recurred: Some("2".to_string()),
	};
	DrugRecurrenceInformationBmc::update(&ctx, &mm, recurrence_id, recurrence_u)
		.await?;
	let recurrence =
		DrugRecurrenceInformationBmc::get(&ctx, &mm, recurrence_id).await?;
	assert_eq!(recurrence.rechallenge_action.as_deref(), Some("1"));
	assert_eq!(recurrence.reaction_recurred.as_deref(), Some("2"));

	let recurrences =
		DrugRecurrenceInformationBmc::list(&ctx, &mm, None, None).await?;
	assert!(recurrences.iter().any(|r| r.id == recurrence_id));

	DrugRecurrenceInformationBmc::delete(&ctx, &mm, recurrence_id).await?;
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}
