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
	let body = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
	Ok((status, body))
}

#[serial]
#[tokio::test]
async fn editor_shell_returns_only_case_header_workflow_and_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let safety_report_id = format!("EDITOR-SHELL-{}", Uuid::new_v4());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safety_report_id": safety_report_id,
				"status": "draft",
				"dg_prd_key": "DG-EDITOR-SHELL"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let case_id = body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/shell"))
			.await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["id"], case_id);
	assert!(body.get("status").is_some());
	assert!(body.get("appendices").is_some());
	assert!(body.get("canActOnWorkflow").is_some());
	assert!(body.get("reactions").is_none());
	assert!(body.get("testResults").is_none());
	assert!(body.get("drugs").is_none());
	assert!(body.get("patientInformation").is_none());
	assert!(body.get("messageHeader").is_none());
	assert!(body.get("safetyReportIdentification").is_none());

	Ok(())
}
