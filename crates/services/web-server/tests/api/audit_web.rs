use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serial_test::serial;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_audit_list_requires_permission() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&viewer_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}
