use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_bad_uuid_returns_400() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri("/api/users/not-a-uuid")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::BAD_REQUEST);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_not_found_returns_404() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let missing_id = Uuid::new_v4();

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/users/{missing_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::NOT_FOUND);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_missing_session_returns_401_with_request_reference() -> Result<()> {
	let mm = init_test_mm().await?;
	let response = web_server::app(mm)
		.oneshot(
			Request::builder()
				.uri("/api/users/me")
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	let request_id = response
		.headers()
		.get("x-request-id")
		.ok_or("missing request id")?
		.to_str()?
		.to_owned();
	let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
	let value: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(value["error"]["message"], "NO_AUTH");
	assert_eq!(value["error"]["data"]["req_uuid"], request_id);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_auth_database_failure_preserves_session_cookie() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	// Close a separate real pool; the shared test pool and database stay available.
	let unavailable = lib_core::model::ModelManager::new().await?;
	unavailable.dbx().db().close().await;
	let response = web_server::app(unavailable)
		.oneshot(
			Request::builder()
				.uri("/api/users/me")
				.header("cookie", cookie_header(&token.to_string()))
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
	assert!(response
		.headers()
		.get_all("set-cookie")
		.iter()
		.next()
		.is_none());
	let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
	let value: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(value["error"]["message"], "SERVICE_ERROR");
	assert!(value["error"]["data"]["detail"].is_null());
	assert!(!String::from_utf8(body.to_vec())?.contains("PoolClosed"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_real_postgres_unknown_sqlstate_is_not_invalid_input() -> Result<()> {
	use axum::response::IntoResponse;
	use lib_web::middleware::{mw_req_stamp::ReqStamp, mw_res_map::mw_response_map};
	let mm = init_test_mm().await?;
	let database_error = sqlx::query("SELECT error_contract_private_missing_column")
		.execute(mm.dbx().db())
		.await
		.expect_err("missing column must fail in PostgreSQL");
	assert_eq!(
		database_error
			.as_database_error()
			.and_then(|error| error.code())
			.as_deref(),
		Some("42703")
	);
	let error = lib_rest_core::Error::Model(lib_core::model::Error::Dbx(
		lib_core::model::store::dbx::Error::Sqlx(database_error),
	));
	let request_id = Uuid::new_v4();
	let response = mw_response_map(
		Err(lib_web::Error::ReqStampNotInReqExt),
		"/api/users".parse()?,
		axum::http::Method::GET,
		ReqStamp {
			uuid: request_id,
			time_in: lib_utils::time::now_utc(),
		},
		None,
		error.into_response(),
	)
	.await;
	assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
	let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
	let value: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(value["error"]["message"], "SERVICE_ERROR");
	assert!(value["error"]["data"]["detail"].is_null());
	assert_eq!(value["error"]["data"]["req_uuid"], request_id.to_string());
	assert!(!String::from_utf8(body.to_vec())?.contains("error_contract_private"));
	Ok(())
}
