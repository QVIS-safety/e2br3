use super::validation_common::{
	assert_has_code, create_message_header, create_safety_report, put_json,
	setup_case, validate_case,
};
use crate::common::Result;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn n_section_batch_number_is_required() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.N.1.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn n_section_batch_sender_identifier_is_required() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.N.1.3.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn n_section_batch_receiver_identifier_is_required() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, None).await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.N.1.4.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn n_section_batch_transmission_date_is_required() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, None).await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.N.1.5.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn n_section_message_sender_identifier_is_required() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/message-header", ctx.case_id),
		json!({"data": {"message_sender_identifier": "   "}}),
	)
	.await?;
	if status != axum::http::StatusCode::OK {
		return Err(format!(
			"update message-header failed: status={status} body={body}"
		)
		.into());
	}

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.N.2.r.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn n_section_message_receiver_identifier_is_required() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/message-header", ctx.case_id),
		json!({"data": {"message_receiver_identifier": ""}}),
	)
	.await?;
	if status != axum::http::StatusCode::OK {
		return Err(format!(
			"update message-header failed: status={status} body={body}"
		)
		.into());
	}

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.N.2.r.3.REQUIRED");
	Ok(())
}
