use super::validation_common::{
	assert_banner_issue, assert_section_rule_coverage, create_message_header,
	create_safety_report, create_test_result, db_exec_case_sql, put_json,
	setup_case, validate_case,
};
use crate::common::Result;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

pub(crate) fn tested_rule_codes() -> &'static [&'static str] {
	&[
		"ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
		"ICH.F.r.2.REQUIRED",
		"ICH.F.r.2.1.REQUIRED",
		"ICH.F.r.2.2a.REQUIRED",
		"ICH.F.r.2.2b.REQUIRED",
		"ICH.F.r.3.1.REQUIRED",
		"ICH.F.r.3.2.REQUIRED",
		"ICH.F.r.3.3.REQUIRED",
		"ICH.F.r.3.4.REQUIRED",
	]
}

#[test]
fn f_rule_coverage_matches_backend_banner_contract() {
	assert_section_rule_coverage('F', tested_rule_codes());
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_1_future_date_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	db_exec_case_sql(
		&ctx,
		&format!("UPDATE test_results SET test_date = '2999-01-01' WHERE id = '{test_id}'"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.F.r.1.FUTURE_DATE.FORBIDDEN");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_2_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE test_results SET test_name = '', test_date = '2024-01-01' WHERE id = '{test_id}'"
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.F.r.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_2_2a_required_returns_banner_issue() -> Result<()> {
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
	assert_banner_issue(&report, "ICH.F.r.2.2a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_2_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE test_results SET test_name = '', test_date = '2024-01-01', test_meddra_code = NULL, test_meddra_version = NULL WHERE id = '{test_id}'"
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.F.r.2.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_2_2b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/test-results/{test_id}", ctx.case_id),
		json!({"data": { "test_date": "2024-01-01", "test_name": "", "test_meddra_code": "", "test_meddra_version": "" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	db_exec_case_sql(
		&ctx,
		&format!("UPDATE test_results SET test_name = '', test_meddra_code = '', test_meddra_version = '', test_date = '2024-01-01' WHERE id = '{test_id}'"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.F.r.2.2b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_3_1_required_returns_banner_issue() -> Result<()> {
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
			"UPDATE test_results SET test_result_code = NULL, test_result_value = NULL, test_result_unit = NULL, result_unstructured = NULL WHERE id = '{test_id}'"
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.F.r.3.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_3_2_required_returns_banner_issue() -> Result<()> {
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
			"UPDATE test_results SET test_result_code = NULL, test_result_value = NULL, test_result_unit = NULL, result_unstructured = NULL WHERE id = '{test_id}'"
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.F.r.3.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_3_3_required_returns_banner_issue() -> Result<()> {
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
	assert_banner_issue(&report, "ICH.F.r.3.3.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_ich_f_r_3_4_required_returns_banner_issue() -> Result<()> {
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
			"UPDATE test_results SET test_result_code = NULL, test_result_value = NULL, test_result_unit = NULL, result_unstructured = NULL WHERE id = '{test_id}'"
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.F.r.3.4.REQUIRED");
	Ok(())
}
