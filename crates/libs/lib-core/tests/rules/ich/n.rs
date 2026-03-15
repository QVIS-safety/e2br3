use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_lacks_issue, create_case_with_safety_report,
	validate_case,
};
use lib_core::model::message_header::{
	MessageHeaderBmc, MessageHeaderForCreate, MessageHeaderForUpdate,
};
use lib_core::xml::validate::ValidationProfile;
use serial_test::serial;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;

fn blank_message_header_update() -> MessageHeaderForUpdate {
	MessageHeaderForUpdate {
		batch_number: None,
		batch_sender_identifier: None,
		batch_receiver_identifier: None,
		batch_transmission_date: None,
		message_number: None,
		message_sender_identifier: None,
		message_receiver_identifier: None,
		message_date: None,
	}
}

async fn create_header_case(
) -> Result<(lib_core::ctx::Ctx, lib_core::model::ModelManager, Uuid)> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	Ok((ctx, mm, case_id))
}

async fn create_message_header(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	case_id: Uuid,
	message_sender_identifier: &str,
	message_receiver_identifier: &str,
) -> Result<()> {
	let message_number = format!("MSG-{}", Uuid::new_v4());
	MessageHeaderBmc::create(
		ctx,
		mm,
		MessageHeaderForCreate {
			case_id,
			message_number,
			message_sender_identifier: message_sender_identifier.to_string(),
			message_receiver_identifier: message_receiver_identifier.to_string(),
			message_date: "20260101000000".to_string(),
		},
	)
	.await?;

	Ok(())
}

async fn populate_batch_fields(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	case_id: Uuid,
) -> Result<()> {
	let mut header_u = blank_message_header_update();
	header_u.batch_number = Some("BATCH-001".to_string());
	header_u.batch_sender_identifier = Some("BATCH-SENDER".to_string());
	header_u.batch_receiver_identifier = Some("BATCH-RECEIVER".to_string());
	header_u.batch_transmission_date = Some(OffsetDateTime::now_utc());
	MessageHeaderBmc::update_by_case(ctx, mm, case_id, header_u).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_1_2_required_false() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.N.1.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_1_2_required_true() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.N.1.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_1_3_required_false() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.N.1.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_1_3_required_true() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.N.1.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_1_4_required_false() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.N.1.4.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_1_4_required_true() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.N.1.4.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_1_5_required_false() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.N.1.5.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_1_5_required_true() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.N.1.5.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_2_r_2_required_false() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "", "MSG-RECEIVER").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.N.2.r.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_2_r_2_required_true() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.N.2.r.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_2_r_3_required_false() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.N.2.r.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_2_r_3_required_true() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.N.2.r.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_required_false() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.N.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_n_required_true() -> Result<()> {
	let (ctx, mm, case_id) = create_header_case().await?;
	create_message_header(&ctx, &mm, case_id, "MSG-SENDER", "MSG-RECEIVER").await?;
	populate_batch_fields(&ctx, &mm, case_id).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.N.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}
