use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::{CaseBmc, CaseFilter, CaseForUpdate};
use modql::filter::{OpValString, OpValsString};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_case_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let case = CaseBmc::get(&ctx, &mm, case_id).await?;
	assert_eq!(case.id, case_id);

	let cases = CaseBmc::list(
		&ctx,
		&mm,
		Some(vec![CaseFilter {
			organization_id: None,
			safety_report_id: Some(OpValsString::from(vec![OpValString::Eq(
				case.safety_report_id.clone(),
			)])),
			status: None,
		}]),
		None,
	)
	.await?;
	assert!(cases.iter().any(|c| c.id == case_id));

	let case_u = CaseForUpdate {
		safety_report_id: None,
		dg_prd_key: None,
		status: Some("validated".to_string()),
		validation_profile: None,
		submitted_by: None,
		submitted_at: None,
		raw_xml: None,
		dirty_c: None,
		dirty_d: None,
		dirty_e: None,
		dirty_f: None,
		dirty_g: None,
		dirty_h: None,
	};
	CaseBmc::update(&ctx, &mm, case_id, case_u).await?;
	let case = CaseBmc::get(&ctx, &mm, case_id).await?;
	assert_eq!(case.status, "validated");

	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
