use crate::common::{
	cookie_header, init_test_mm, seed_org_with_all_roles, seed_org_with_users,
	Result,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serial_test::serial;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_admin_can_list_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_list_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_manager_can_list_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&manager_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_and_viewer_cannot_list_audit_logs() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [("user", user_token), ("viewer", viewer_token)] {
		let req = Request::builder()
			.method("GET")
			.uri("/api/audit-logs")
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from listing audit logs"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_manager_can_list_audit_logs_by_record() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let manager_token =
		generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let req = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/audit-logs/by-record/organizations/{}",
			seed.org_id
		))
		.header("cookie", cookie_header(&manager_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_and_viewer_cannot_list_audit_logs_by_record() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let app = web_server::app(mm);

	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	for (role, token) in [("user", user_token), ("viewer", viewer_token)] {
		let req = Request::builder()
			.method("GET")
			.uri(format!(
				"/api/audit-logs/by-record/organizations/{}",
				seed.org_id
			))
			.header("cookie", cookie_header(&token.to_string()))
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(
			res.status(),
			StatusCode::FORBIDDEN,
			"{role} should be forbidden from reading audit logs by record"
		);
	}

	Ok(())
}
