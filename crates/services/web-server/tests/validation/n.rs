use super::validation_common::{
	assert_banner_issue, assert_section_rule_coverage, create_message_header,
	put_json, setup_case, validate_case,
};
use crate::common::Result;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

pub(crate) fn tested_rule_codes() -> &'static [&'static str] {
	&["ICH.N.1.5.REQUIRED"]
}

#[test]
fn n_rule_coverage_matches_backend_banner_contract() {
	assert_section_rule_coverage('N', tested_rule_codes());
}

#[serial]
#[tokio::test]
async fn n_ich_n_1_5_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/message-header", ctx.case_id),
		json!({"data": {
			"batch_transmission_date": null
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.N.1.5.REQUIRED");
	Ok(())
}
