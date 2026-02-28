use super::validation_common::{
	assert_has_code, create_message_header, create_primary_source,
	create_safety_report, create_sender, setup_case, validate_case,
};
use crate::common::Result;
use lib_core::xml::validate::rule_test_matrix::CASE_RULE_TEST_MATRIX;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn l2_fda_case_matrix_smoke_rule_is_enforced() -> Result<()> {
	assert!(CASE_RULE_TEST_MATRIX
		.iter()
		.any(|s| s.code == "FDA.C.2.r.2.EMAIL.REQUIRED"));

	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.C.2.r.2.EMAIL.REQUIRED");
	Ok(())
}
