use super::validation_common::{
	assert_has_code, create_drug, create_message_header, create_patient,
	create_primary_source, create_reaction, create_safety_report, create_sender,
	setup_case, update_drug, validate_case,
};
use crate::common::Result;
use lib_core::xml::validate::rule_test_matrix::CASE_RULE_TEST_MATRIX;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn l3_mfds_case_matrix_smoke_rule_is_enforced() -> Result<()> {
	assert!(CASE_RULE_TEST_MATRIX
		.iter()
		.any(|s| s.code == "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED"));

	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZMFDS"))
		.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "obtain_drug_country": "KR", "mpid": null }}),
	)
	.await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED");
	Ok(())
}
