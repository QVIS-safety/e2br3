use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_lacks_issue, create_case_with_safety_report,
	validate_case,
};
use lib_core::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};
use lib_core::validation::ValidationProfile;
use serial_test::serial;
use sqlx::types::time::Date;
use time::Month;

fn sample_date() -> Date {
	Date::from_calendar_date(2024, Month::January, 1)
		.expect("sample date should be valid")
}

fn blank_test_result_update() -> TestResultForUpdate {
	TestResultForUpdate {
		test_name: None,
		test_date: None,
		test_date_null_flavor: None,
		test_meddra_version: None,
		test_meddra_code: None,
		test_result_code: None,
		test_result_value: None,
		test_result_unit: None,
		result_unstructured: None,
		normal_low_value: None,
		normal_high_value: None,
		comments: None,
		more_info_available: None,
	}
}

async fn create_test_result_case(
	test_name: &str,
) -> Result<(
	lib_core::ctx::Ctx,
	lib_core::model::ModelManager,
	sqlx::types::Uuid,
	sqlx::types::Uuid,
)> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let test_id = TestResultBmc::create(
		&ctx,
		&mm,
		TestResultForCreate {
			case_id,
			sequence_number: 1,
			test_name: test_name.to_string(),
		},
	)
	.await?;

	Ok((ctx, mm, case_id, test_id))
}

#[serial]
#[tokio::test]
async fn ich_f_r_1_required_false() -> Result<()> {
	let (ctx, mm, case_id, _test_id) = create_test_result_case("ALT").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_date = Some(sample_date());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_2_1_required_false() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_date = Some(sample_date());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.2.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_2_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_date = Some(sample_date());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.2.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_2_2a_required_false() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_meddra_code = Some("10019211".to_string());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.2.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_2_2a_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_meddra_code = Some("10019211".to_string());
	test_u.test_meddra_version = Some("27.0".to_string());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.2.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_2_2b_required_false() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_date = Some(sample_date());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.2.2b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_2_2b_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_date = Some(sample_date());
	test_u.test_meddra_code = Some("10019211".to_string());
	test_u.test_meddra_version = Some("27.0".to_string());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.2.2b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_2_required_false() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_date = Some(sample_date());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_2_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_date = Some(sample_date());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_3_1_required_false() -> Result<()> {
	let (ctx, mm, case_id, _test_id) = create_test_result_case("ALT").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.3.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_3_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_result_code = Some("N".to_string());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.3.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_3_2_required_false() -> Result<()> {
	let (ctx, mm, case_id, _test_id) = create_test_result_case("ALT").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.3.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_3_2_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_result_value = Some("Normal".to_string());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.3.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_3_3_required_false() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_result_value = Some("5".to_string());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.3.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_3_3_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.test_result_value = Some("5".to_string());
	test_u.test_result_unit = Some("mg/dL".to_string());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.3.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_3_4_required_false() -> Result<()> {
	let (ctx, mm, case_id, _test_id) = create_test_result_case("ALT").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.F.r.3.4.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_f_r_3_4_required_true() -> Result<()> {
	let (ctx, mm, case_id, test_id) = create_test_result_case("ALT").await?;
	let mut test_u = blank_test_result_update();
	test_u.result_unstructured = Some("Baseline result".to_string());
	TestResultBmc::update_in_case(&ctx, &mm, case_id, test_id, test_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.F.r.3.4.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}
