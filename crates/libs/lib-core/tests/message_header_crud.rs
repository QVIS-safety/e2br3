mod common;

use common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::message_header::{
	MessageHeaderBmc, MessageHeaderForCreate, MessageHeaderForUpdate,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_message_header_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let header_c = MessageHeaderForCreate {
		case_id,
		message_number: format!("MSG-{case_id}"),
		message_sender_identifier: "SENDER-1".to_string(),
		message_receiver_identifier: "RECEIVER-1".to_string(),
		message_date: "20240101120000".to_string(),
	};
	let header_id = MessageHeaderBmc::create(&ctx, &mm, header_c).await?;
	let header = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(header.id, header_id);

	let header_u = MessageHeaderForUpdate {
		batch_number: Some("B-001".to_string()),
		batch_sender_identifier: None,
		batch_receiver_identifier: None,
		batch_transmission_date: None,
		message_number: Some("MSG-002".to_string()),
		message_sender_identifier: None,
		message_receiver_identifier: None,
		message_date: None,
	};
	MessageHeaderBmc::update_by_case(&ctx, &mm, case_id, header_u).await?;
	let header = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(header.message_number, "MSG-002");

	MessageHeaderBmc::delete_by_case(&ctx, &mm, case_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
