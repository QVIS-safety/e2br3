use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::Value;
use serial_test::serial;
use tower::ServiceExt;

async fn get_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

#[serial]
#[tokio::test]
async fn test_app_branding_uses_default_name_without_e2br3() -> Result<()> {
	std::env::remove_var("E2BR3_APP_NAME");
	std::env::remove_var("E2BR3_APP_SHORT_NAME");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = get_json(&app, &cookie, "/api/app/branding").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["appName"].as_str(), Some("QVIS Safety"));
	assert_eq!(body["data"]["appShortName"].as_str(), Some("QVIS Safety"));
	assert!(!body["data"]["appName"]
		.as_str()
		.unwrap_or_default()
		.contains("E2BR3"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_app_branding_uses_configured_name() -> Result<()> {
	std::env::set_var("E2BR3_APP_NAME", "Qaris");
	std::env::set_var("E2BR3_APP_SHORT_NAME", "Qaris");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = get_json(&app, &cookie, "/api/app/branding").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["appName"].as_str(), Some("Qaris"));
	assert_eq!(body["data"]["appShortName"].as_str(), Some("Qaris"));

	std::env::remove_var("E2BR3_APP_NAME");
	std::env::remove_var("E2BR3_APP_SHORT_NAME");
	Ok(())
}
