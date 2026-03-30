use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use crate::presave_authoring::{create_case, create_template, request_json};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_other_org_presave_cannot_be_loaded_for_authoring_import() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed_a = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let seed_b = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token_a = generate_web_token(&seed_a.admin.email, seed_a.admin.token_salt)?;
	let token_b = generate_web_token(&seed_b.admin.email, seed_b.admin.token_salt)?;
	let cookie_a = cookie_header(&token_a.to_string());
	let cookie_b = cookie_header(&token_b.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie_a,
		"sender",
		"sender-org-a-only",
		json!({
			"senderType": "1",
			"senderOrganization": "Org A Sender"
		}),
	)
	.await?;
	let _case_id = create_case(&app, &cookie_b, seed_b.org_id).await?;

	let (status, value) = request_json(
		&app,
		&cookie_b,
		Method::GET,
		format!("/api/presave-templates/{template_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{value:?}");
	Ok(())
}
