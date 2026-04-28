use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn put_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

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
async fn test_public_case_create_derives_org_and_version() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let attacker_org_id = Uuid::new_v4();

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"organization_id": attacker_org_id,
				"version": 99,
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
				"status": "draft"
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let expected_org_id = seed.org_id.to_string();
	assert_eq!(
		body["data"]["organization_id"].as_str(),
		Some(expected_org_id.as_str()),
		"{body:?}"
	);
	assert_eq!(body["data"]["version"].as_i64(), Some(1), "{body:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_public_case_create_derives_profile_from_appendices() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
				"status": "draft",
				"appendices_json": "[\"mfds\",\"fda\"]"
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert_eq!(body["data"]["validation_profile"], "mfds", "{body:?}");
	assert_eq!(
		body["data"]["appendices_json"], "[\"mfds\",\"fda\"]",
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_public_case_update_ignores_system_managed_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (create_status, create_body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(create_status, StatusCode::CREATED, "{create_body:?}");
	let case_id = create_body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let bogus_submitter = Uuid::new_v4();
	let (update_status, update_body) = put_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}"),
		json!({
			"data": {
				"submitted_by": bogus_submitter,
				"submitted_at": "2026-04-13T00:00:00Z",
				"raw_xml": "ZmFrZQ==",
				"dirty_c": true,
				"dirty_d": true,
				"dirty_e": true,
				"dirty_f": true,
				"dirty_g": true,
				"dirty_h": true
			}
		}),
	)
	.await?;
	assert_eq!(update_status, StatusCode::OK, "{update_body:?}");

	let (get_status, get_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(get_status, StatusCode::OK, "{get_body:?}");
	assert_eq!(
		get_body["data"]["submitted_by"],
		Value::Null,
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["submitted_at"],
		Value::Null,
		"{get_body:?}"
	);
	assert_eq!(get_body["data"]["raw_xml"], Value::Null, "{get_body:?}");
	assert_eq!(
		get_body["data"]["dirty_c"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_d"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_e"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_f"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_g"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["dirty_h"].as_bool(),
		Some(false),
		"{get_body:?}"
	);
	assert_eq!(
		get_body["data"]["status"].as_str(),
		Some("draft"),
		"{get_body:?}"
	);
	Ok(())
}
