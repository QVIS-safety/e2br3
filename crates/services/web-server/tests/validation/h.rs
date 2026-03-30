use super::validation_common::{
	assert_banner_issue, assert_section_rule_coverage, create_message_header,
	create_narrative, create_patient, create_primary_source, create_reaction,
	create_safety_report, create_sender, create_sender_diagnosis, put_json,
	setup_case, update_narrative, validate_case,
};
use crate::common::Result;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

pub(crate) fn tested_rule_codes() -> &'static [&'static str] {
	&[
		"ICH.H.1.REQUIRED",
		"ICH.H.3.r.1a.REQUIRED",
		"ICH.H.3.r.1b.REQUIRED",
	]
}

#[test]
fn h_rule_coverage_matches_backend_banner_contract() {
	assert_section_rule_coverage('H', tested_rule_codes());
}

#[serial]
#[tokio::test]
async fn h_ich_h_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1")).await?;
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
	assert_banner_issue(&report, "ICH.H.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn h_ich_h_3_r_1a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1")).await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let narrative_id =
		create_narrative(&ctx.app, &ctx.cookie, ctx.case_id, "Narrative").await?;
	let sender_diagnosis_id = create_sender_diagnosis(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		narrative_id,
		1,
		Some("10012345"),
	)
	.await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!(
			"/api/cases/{}/narrative/sender-diagnoses/{sender_diagnosis_id}",
			ctx.case_id
		),
		json!({"data": { "diagnosis_meddra_version": "" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.H.3.r.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn h_ich_h_3_r_1b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1")).await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let narrative_id =
		create_narrative(&ctx.app, &ctx.cookie, ctx.case_id, "Narrative").await?;
	let sender_diagnosis_id = create_sender_diagnosis(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		narrative_id,
		1,
		Some("10012345"),
	)
	.await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!(
			"/api/cases/{}/narrative/sender-diagnoses/{sender_diagnosis_id}",
			ctx.case_id
		),
		json!({"data": { "diagnosis_meddra_version": "27.0", "diagnosis_meddra_code": "" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.H.3.r.1b.REQUIRED");
	Ok(())
}
