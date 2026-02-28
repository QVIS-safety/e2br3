use super::validation_common::{
	create_message_header, create_primary_source, create_safety_report,
	create_sender, create_test_result, put_json, setup_case,
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
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
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
