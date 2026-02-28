use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberForCreate, LinkedReportNumberForUpdate,
	OtherCaseIdentifierBmc, OtherCaseIdentifierForCreate,
	OtherCaseIdentifierForUpdate,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_case_identifiers_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let other_c = OtherCaseIdentifierForCreate {
		case_id,
		sequence_number: 1,
		source_of_identifier: "EMA".to_string(),
		case_identifier: "EMA-001".to_string(),
	};
	let other_id = OtherCaseIdentifierBmc::create(&ctx, &mm, other_c).await?;
	let other = OtherCaseIdentifierBmc::get(&ctx, &mm, other_id).await?;
	assert_eq!(other.case_identifier, "EMA-001");

	let other_u = OtherCaseIdentifierForUpdate {
		source_of_identifier: None,
		case_identifier: Some("EMA-002".to_string()),
	};
	OtherCaseIdentifierBmc::update(&ctx, &mm, other_id, other_u).await?;
	let other = OtherCaseIdentifierBmc::get(&ctx, &mm, other_id).await?;
	assert_eq!(other.case_identifier, "EMA-002");

	let others = OtherCaseIdentifierBmc::list(&ctx, &mm, None, None).await?;
	assert!(others.iter().any(|o| o.id == other_id));

	let linked_c = LinkedReportNumberForCreate {
		case_id,
		sequence_number: 1,
		linked_report_number: "LR-001".to_string(),
	};
	let linked_id = LinkedReportNumberBmc::create(&ctx, &mm, linked_c).await?;
	let linked = LinkedReportNumberBmc::get(&ctx, &mm, linked_id).await?;
	assert_eq!(linked.linked_report_number, "LR-001");

	let linked_u = LinkedReportNumberForUpdate {
		linked_report_number: Some("LR-002".to_string()),
	};
	LinkedReportNumberBmc::update(&ctx, &mm, linked_id, linked_u).await?;
	let linked = LinkedReportNumberBmc::get(&ctx, &mm, linked_id).await?;
	assert_eq!(linked.linked_report_number, "LR-002");

	let linked_reports = LinkedReportNumberBmc::list(&ctx, &mm, None, None).await?;
	assert!(linked_reports.iter().any(|l| l.id == linked_id));

	LinkedReportNumberBmc::delete(&ctx, &mm, linked_id).await?;
	OtherCaseIdentifierBmc::delete(&ctx, &mm, other_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}
