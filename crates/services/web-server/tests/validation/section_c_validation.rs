use super::validation_common::{
	assert_has_code, create_message_header, create_primary_source,
	create_safety_report, create_sender, setup_case, update_primary_source,
	validate_case,
};
use crate::common::Result;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn c_section_required_when_case_header_blocks_are_missing() -> Result<()> {
	let ctx = setup_case().await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "ICH.C.1.REQUIRED");
	assert_has_code(&report, "ICH.N.REQUIRED");
	assert_has_code(&report, "ICH.C.3.1.REQUIRED");
	assert_has_code(&report, "ICH.C.3.2.REQUIRED");
	assert_has_code(&report, "ICH.C.2.r.4.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_section_primary_source_qualification_is_validated() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	let ps_id =
		create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, None).await?;
	update_primary_source(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		ps_id,
		json!({"data": { "organization": "Reporter Org" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "ICH.C.2.r.4.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_section_fda_local_criteria_and_reporter_email_are_validated() -> Result<()>
{
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.C.1.7.1.REQUIRED");
	assert_has_code(&report, "FDA.C.2.r.2.EMAIL.REQUIRED");
	assert_has_code(&report, "FDA.C.1.12.RECOMMENDED");
	Ok(())
}
