use super::validation_common::{
	assert_banner_issue, assert_section_rule_coverage, create_message_header,
	create_primary_source, create_reaction, create_safety_report, create_sender,
	setup_case, update_reaction, validate_case,
};
use crate::common::Result;
use serde_json::json;
use serial_test::serial;

pub(crate) fn tested_rule_codes() -> &'static [&'static str] {
	&[
		"ICH.E.i.1.1a.REQUIRED",
		"ICH.E.i.2.1a.REQUIRED",
		"ICH.E.i.2.1b.REQUIRED",
		"ICH.E.i.6a.REQUIRED",
		"ICH.E.i.6b.REQUIRED",
		"ICH.E.i.7.REQUIRED",
		"FDA.E.i.3.2h.REQUIRED",
	]
}

#[test]
fn e_rule_coverage_matches_backend_banner_contract() {
	assert_section_rule_coverage('E', tested_rule_codes());
}

#[serial]
#[tokio::test]
async fn e_ich_e_i_1_1a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	update_reaction(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		reaction_id,
		json!({"data": { "primary_source_reaction": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.E.i.1.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn e_ich_e_i_2_1a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	update_reaction(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		reaction_id,
		json!({"data": { "reaction_meddra_code": "10027940", "reaction_meddra_version": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.E.i.2.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn e_ich_e_i_2_1b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.E.i.2.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn e_ich_e_i_6a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	update_reaction(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		reaction_id,
		json!({"data": { "duration_value": null, "duration_unit": "d" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.E.i.6a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn e_ich_e_i_6b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	update_reaction(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		reaction_id,
		json!({"data": { "duration_value": "5", "duration_unit": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.E.i.6b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn e_ich_e_i_7_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.E.i.7.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn e_fda_e_i_3_2h_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	update_reaction(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		reaction_id,
		json!({"data": {
			"criteria_other_medically_important": true,
			"required_intervention": null
		}}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_banner_issue(&report, "FDA.E.i.3.2h.REQUIRED");
	Ok(())
}
