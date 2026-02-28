use super::validation_common::{
	assert_has_code, create_message_header, create_narrative, create_patient,
	create_primary_source, create_reaction, create_safety_report, create_sender,
	setup_case, update_narrative, validate_case,
};
use crate::common::Result;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn h_section_case_narrative_rule_is_enforced_when_narrative_missing(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.H.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn h_section_case_narrative_rule_is_enforced_when_comments_present(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	create_narrative(&ctx.app, &ctx.cookie, ctx.case_id, "").await?;
	update_narrative(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "reporter_comments": "payload present", "case_narrative": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.H.1.REQUIRED");
	Ok(())
}
