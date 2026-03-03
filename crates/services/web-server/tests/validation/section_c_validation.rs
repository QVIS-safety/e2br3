use super::validation_common::{
	assert_has_code, create_message_header, create_message_header_with_receiver,
	create_primary_source, create_safety_report, create_safety_report_with,
	create_sender, create_study_information, create_study_registration, setup_case,
	update_primary_source, validate_case,
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

#[serial]
#[tokio::test]
async fn c_section_fda_study_ind_number_is_required_for_cder_ind() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report_with(&ctx.app, &ctx.cookie, ctx.case_id, "1", false)
		.await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZFDA_PREMKT"),
		"CDER_IND",
	)
	.await?;
	create_study_information(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("Study"),
		Some("ABC123"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.C.5.5a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_section_fda_pre_anda_number_is_required_for_ind_exempt_receiver(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report_with(&ctx.app, &ctx.cookie, ctx.case_id, "2", false)
		.await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZFDA_PREMKT"),
		"CDER_IND_EXEMPT_BA_BE",
	)
	.await?;
	create_study_information(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("Study"),
		Some("A23456"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.C.5.5b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_section_fda_cross_reported_ind_required_when_ind_is_populated(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report_with(&ctx.app, &ctx.cookie, ctx.case_id, "1", false)
		.await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZFDA_PREMKT"),
		"CBER_IND",
	)
	.await?;
	let study_id = create_study_information(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("Study"),
		Some("123456"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.C.5.6.r.REQUIRED");

	create_study_registration(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		study_id,
		1,
		"654321",
		Some("US"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert!(!super::validation_common::issue_codes(&report)
		.iter()
		.any(|c| c == "FDA.C.5.6.r.REQUIRED"));
	Ok(())
}
