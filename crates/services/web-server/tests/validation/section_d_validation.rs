use super::validation_common::{
	assert_has_code, create_message_header, create_message_header_with_receiver,
	create_parent_information, create_parent_past_drug_history,
	create_past_drug_history, create_patient, create_primary_source,
	create_safety_report, create_sender, setup_case,
	update_parent_past_drug_history, update_patient, validate_case,
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

#[serial]
#[tokio::test]
async fn d_section_fda_race_and_ethnicity_reject_invalid_codes() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("1"), Some("1")).await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.D.11.REQUIRED");
	assert_has_code(&report, "FDA.D.12.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_mfds_past_drug_code_required_for_kr_receiver() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"KR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let _ = create_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		1,
		Some("Past Drug"),
		None,
		None,
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.D.8.r.1.KR.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_mfds_parent_past_drug_version_required_for_fr_when_code_present(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"FR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let parent_id =
		create_parent_information(&ctx.app, &ctx.cookie, ctx.case_id, Some("2"))
			.await?;
	let parent_past_id = create_parent_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		parent_id,
		1,
		Some("Parent Past Drug"),
	)
	.await?;
	update_parent_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		parent_id,
		parent_past_id,
		json!({"data": { "mpid": "WHOMPID-001", "mpid_version": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.D.10.8.r.1.KR.1a.REQUIRED");
	Ok(())
}
