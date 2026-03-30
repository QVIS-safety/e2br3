use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use crate::presave_authoring::{
	apply_authoring_presave, create_case, create_template, get_template_data,
};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_receiver_presave_is_rejected_from_authoring_import_flow() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie,
		"receiver",
		"receiver-not-authoring",
		json!({
			"receiverType": "2",
			"organizationName": "Submission Receiver Org"
		}),
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let err =
		apply_authoring_presave(&app, &cookie, case_id, "receiver", &saved_data)
			.await
			.expect_err("receiver should not import into case authoring");
	assert!(err
		.to_string()
		.contains("unsupported authoring presave entity_type"));
	Ok(())
}
