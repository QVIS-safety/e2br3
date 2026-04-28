use super::common::{date, finish, setup_case};
use crate::test_common::Result;
use lib_core::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};
use serial_test::serial;
use time::Month;

#[tokio::test]
#[serial]
async fn save_f_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = TestResultBmc::create(
		&ctx,
		&mm,
		TestResultForCreate {
			case_id,
			sequence_number: 1,
			test_name: "ALT".to_string(),
			test_date: Some(date(2024, Month::January, 1)),
			test_date_null_flavor: None,
			test_meddra_version: Some("27.0".to_string()),
			test_meddra_code: Some("1000".to_string()),
			test_result_code: Some("N".to_string()),
			test_result_value: Some("11".to_string()),
			test_result_unit: Some("mg/dL".to_string()),
			result_unstructured: Some("Normal".to_string()),
			normal_low_value: Some("1".to_string()),
			normal_high_value: Some("20".to_string()),
			comments: Some("Comment".to_string()),
			more_info_available: Some(true),
		},
	)
	.await?;
	let row = TestResultBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.test_date, Some(date(2024, Month::January, 1)));
	assert_eq!(row.test_date_null_flavor, None);
	assert_eq!(row.test_name, "ALT");
	assert_eq!(row.test_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.test_meddra_code.as_deref(), Some("1000"));
	assert_eq!(row.test_result_code.as_deref(), Some("N"));
	assert_eq!(row.test_result_value.as_deref(), Some("11"));
	assert_eq!(row.test_result_unit.as_deref(), Some("mg/dL"));
	assert_eq!(row.result_unstructured.as_deref(), Some("Normal"));
	assert_eq!(row.normal_low_value.as_deref(), Some("1"));
	assert_eq!(row.normal_high_value.as_deref(), Some("20"));
	assert_eq!(row.comments.as_deref(), Some("Comment"));
	assert_eq!(row.more_info_available, Some(true));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_f_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = TestResultBmc::create(
		&ctx,
		&mm,
		TestResultForCreate {
			case_id,
			sequence_number: 1,
			test_name: "ALT".to_string(),
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
		},
	)
	.await?;
	TestResultBmc::update(
		&ctx,
		&mm,
		id,
		TestResultForUpdate {
			test_name: Some("AST".to_string()),
			test_date: Some(date(2024, Month::January, 1)),
			test_date_null_flavor: None,
			test_meddra_version: Some("27.0".to_string()),
			test_meddra_code: Some("1000".to_string()),
			test_result_code: Some("N".to_string()),
			test_result_value: Some("11".to_string()),
			test_result_unit: Some("mg/dL".to_string()),
			result_unstructured: Some("Normal".to_string()),
			normal_low_value: Some("1".to_string()),
			normal_high_value: Some("20".to_string()),
			comments: Some("Comment".to_string()),
			more_info_available: Some(true),
		},
	)
	.await?;
	let row = TestResultBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.test_date, Some(date(2024, Month::January, 1)));
	assert_eq!(row.test_date_null_flavor, None);
	assert_eq!(row.test_name, "AST");
	assert_eq!(row.test_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.test_meddra_code.as_deref(), Some("1000"));
	assert_eq!(row.test_result_code.as_deref(), Some("N"));
	assert_eq!(row.test_result_value.as_deref(), Some("11"));
	assert_eq!(row.test_result_unit.as_deref(), Some("mg/dL"));
	assert_eq!(row.result_unstructured.as_deref(), Some("Normal"));
	assert_eq!(row.normal_low_value.as_deref(), Some("1"));
	assert_eq!(row.normal_high_value.as_deref(), Some("20"));
	assert_eq!(row.comments.as_deref(), Some("Comment"));
	assert_eq!(row.more_info_available, Some(true));
	finish(&mm).await
}
