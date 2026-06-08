#![allow(unused_imports)]

use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_admin_settings_appendices_are_supported_and_never_empty() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"appendices": ["FDA", "MFDS", "ICH"]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["appendices"], json!(["ICH", "FDA", "MFDS"]));

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"appendices": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["appendices"], json!(["ICH"]));

	Ok(())
}
