use crate::administration_support::request_json;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_negative_due_days() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["pvs"],
							"due_days": -1
						}
					]
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("due_days"), "{body:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_unknown_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["not_a_real_role"],
							"due_days": 0
						}
					]
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("unknown role 'not_a_real_role'"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_system_admin_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["system_admin"],
							"due_days": 0
						}
					]
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("unknown role 'system_admin'"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_allow_empty_roles_as_unrestricted() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = request_json(
		&app,
		"PUT",
		&cookie,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": [],
							"due_days": 0
						}
					]
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["workflow"]["statuses"][0]["allowed_roles"], json!([]));
	Ok(())
}
