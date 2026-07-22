use crate::common::{
	cookie_header, init_test_mm, insert_case_version, seed_two_orgs_manager_cases,
	seed_two_orgs_users_cases, system_org_id, system_user_id, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use lib_web::Error as WebError;
use serde_json::Value;
use serial_test::serial;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_users_endpoints_require_admin_guard() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_users_cases(&mm).await?;
	let token = generate_web_token(&seed.user1.email, seed.user1.token_salt)?;
	let cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri("/api/users")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/users/{}", seed.user2.id))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_rls_list_cases_filters_org() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_users_cases(&mm).await?;
	let token = generate_web_token(&seed.user1.email, seed.user1.token_salt)?;
	let cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri("/api/cases")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let cases = value
		.get("data")
		.and_then(|v| v.as_array())
		.ok_or("missing data array")?;

	let case_org1 = seed.case_org1.to_string();
	let case_org2 = seed.case_org2.to_string();
	assert!(cases
		.iter()
		.any(|c| c.get("id").and_then(|v| v.as_str()) == Some(&case_org1)));
	assert!(!cases
		.iter()
		.any(|c| c.get("id").and_then(|v| v.as_str()) == Some(&case_org2)));

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_org2}"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	if status != StatusCode::BAD_REQUEST && status != StatusCode::NOT_FOUND {
		let err = res
			.extensions()
			.get::<std::sync::Arc<WebError>>()
			.map(|e| format!("{e:?}"));
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"case get status {} body {} err {:?}",
			status,
			String::from_utf8_lossy(&body),
			err
		)
		.into());
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_rls_case_versions_filters_org() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_two_orgs_manager_cases(&mm).await?;

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(
		dbx,
		system_user_id(),
		system_org_id(),
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	insert_case_version(&mm, seed.case_org1, 1, seed.manager.id).await?;
	insert_case_version(&mm, seed.case_org2, 1, seed.user2.id).await?;
	dbx.commit_txn().await?;

	let token = generate_web_token(&seed.manager.email, seed.manager.token_salt)?;
	let cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{}/versions", seed.case_org1))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{}/versions", seed.case_org2))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}
