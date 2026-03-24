use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_test_result_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let test_c = TestResultForCreate {
		case_id,
		sequence_number: 1,
		test_name: "Blood Test".to_string(),
	};
	let test_id = TestResultBmc::create(&ctx, &mm, test_c).await?;
	let test = TestResultBmc::get(&ctx, &mm, test_id).await?;
	assert_eq!(test.test_name, "Blood Test");

	let test_u = TestResultForUpdate {
		test_name: Some("Updated Test".to_string()),
		test_date: None,
		test_date_null_flavor: None,
		test_meddra_version: None,
		test_meddra_code: None,
		test_result_code: None,
		test_result_value: Some("Normal".to_string()),
		test_result_unit: None,
		result_unstructured: None,
		normal_low_value: None,
		normal_high_value: None,
		comments: None,
		more_info_available: None,
	};
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;
	let test = TestResultBmc::get_in_case(&ctx, &mm, case_id, test_id).await?;
	assert_eq!(test.test_name, "Updated Test");

	let tests = TestResultBmc::list_by_case(&ctx, &mm, case_id).await?;
	assert!(tests.iter().any(|t| t.id == test_id));

	TestResultBmc::delete(&ctx, &mm, test_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
