use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, unique_suffix, Result,
};
use lib_core::model::organization::{
	OrganizationBmc, OrganizationForCreate, OrganizationForUpdate,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_organization_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let suffix = unique_suffix();
	let org_c = OrganizationForCreate {
		name: format!("Test Org {suffix}"),
		org_type: Some("internal".to_string()),
		address: Some("123 Test St".to_string()),
		contact_email: Some(format!("test-org-{suffix}@example.com")),
	};

	let org_id = OrganizationBmc::create(&ctx, &mm, org_c).await?;
	let org = OrganizationBmc::get(&ctx, &mm, org_id).await?;
	assert_eq!(org.name, format!("Test Org {suffix}"));

	let orgs = OrganizationBmc::list(&ctx, &mm, None, None).await?;
	assert!(orgs.iter().any(|o| o.id == org_id));

	let org_u = OrganizationForUpdate {
		name: Some("Updated Org".to_string()),
		org_type: None,
		address: None,
		city: None,
		state: None,
		postcode: None,
		country_code: None,
		contact_email: None,
		contact_phone: None,
		active: Some(false),
	};
	OrganizationBmc::update(&ctx, &mm, org_id, org_u).await?;
	let org = OrganizationBmc::get(&ctx, &mm, org_id).await?;
	assert_eq!(org.name, "Updated Org");
	assert!(!org.active);

	OrganizationBmc::delete(&ctx, &mm, org_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
