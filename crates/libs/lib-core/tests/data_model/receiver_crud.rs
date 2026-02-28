use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::receiver::{
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_receiver_information_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let receiver_c = ReceiverInformationForCreate {
		case_id,
		receiver_type: Some("2".to_string()),
		organization_name: Some("FDA".to_string()),
	};
	let receiver_id = ReceiverInformationBmc::create(&ctx, &mm, receiver_c).await?;
	let receiver = ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(receiver.id, receiver_id);
	assert_eq!(receiver.organization_name.as_deref(), Some("FDA"));

	let receiver_u = ReceiverInformationForUpdate {
		receiver_type: None,
		organization_name: Some("EMA".to_string()),
		department: Some("Safety".to_string()),
		street_address: None,
		city: None,
		state_province: None,
		postcode: None,
		country_code: Some("DE".to_string()),
		telephone: None,
		fax: None,
		email: Some("safety@example.org".to_string()),
	};
	ReceiverInformationBmc::update_by_case(&ctx, &mm, case_id, receiver_u).await?;
	let receiver = ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(receiver.organization_name.as_deref(), Some("EMA"));
	assert_eq!(receiver.department.as_deref(), Some("Safety"));
	assert_eq!(receiver.country_code.as_deref(), Some("DE"));

	ReceiverInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}
