use crate::common::{seed_active_test_meddra_term, Result};
use crate::validation_common::{
	create_message_header, create_primary_source, create_safety_report,
	create_sender, create_test_result, put_json, setup_case,
};
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn validation_contract_accepts_meddra_test_name_alternative() -> Result<()> {
	let ctx = setup_case().await?;
	seed_active_test_meddra_term(&ctx.mm).await?;
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
		json!({"data": {
			"test_date": "20210303",
			"test_name": "",
			"test_meddra_version": "26.0",
			"test_meddra_code": "10000001"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	Ok(())
}
