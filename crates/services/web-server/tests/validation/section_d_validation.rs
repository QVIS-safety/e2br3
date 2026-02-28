use super::validation_common::{
	assert_has_code, create_message_header, create_patient, create_primary_source,
	create_safety_report, create_sender, setup_case, update_patient, validate_case,
};
use crate::common::Result;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn d_section_patient_initials_rule_is_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, None, Some("1")).await?;
	update_patient(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "patient_given_name": "Jane" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_fda_race_and_ethnicity_rules_are_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.D.11.REQUIRED");
	assert_has_code(&report, "FDA.D.12.REQUIRED");
	Ok(())
}
