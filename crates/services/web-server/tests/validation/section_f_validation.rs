use super::validation_common::{
	assert_has_code, create_message_header, create_safety_report,
	create_test_result, db_exec_case_sql, put_json, setup_case, validate_case,
};
use crate::common::Result;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn f_section_test_name_rejects_blank_value_at_api_layer() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/test-results/{test_id}", ctx.case_id),
		json!({"data": { "test_name": "", "test_result_code": "POS" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_section_test_meddra_version_is_required_when_meddra_code_is_present(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/test-results/{test_id}", ctx.case_id),
		json!({"data": { "test_meddra_code": "10000001", "test_meddra_version": null }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.F.r.2.2a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_section_test_result_unit_is_required_when_result_value_is_present(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/test-results/{test_id}", ctx.case_id),
		json!({"data": { "test_result_value": "5", "test_result_unit": null }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.F.r.3.3.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_section_test_date_is_required_when_test_name_is_present() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/test-results/{test_id}", ctx.case_id),
		json!({"data": { "test_date": null, "test_name": "LFT" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.F.r.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_section_some_result_content_is_required_when_test_name_is_present(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/test-results/{test_id}", ctx.case_id),
		json!({"data": { "test_date": "2024-01-01" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE test_results
			 SET test_result_code = NULL,
			     test_result_value = NULL,
			     test_result_unit = NULL,
			     result_unstructured = NULL
			 WHERE id = '{test_id}'"
		),
	)
	.await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.F.r.3.1.REQUIRED");
	assert_has_code(&report, "ICH.F.r.3.2.REQUIRED");
	assert_has_code(&report, "ICH.F.r.3.4.REQUIRED");
	Ok(())
}
