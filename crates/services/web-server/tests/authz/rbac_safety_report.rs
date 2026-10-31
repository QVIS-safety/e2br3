use crate::authz_support::create_case;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn create_safety_report(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<(StatusCode, String)> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"transmission_date": "20240101000000",
			"report_type": "1",
			"date_first_received_from_source": "20240101",
			"date_of_most_recent_information": "20240101",
			"fulfil_expedited_criteria": false
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/safety-report"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, String::from_utf8_lossy(&body).to_string()))
}

#[serial]
#[tokio::test]
async fn test_safety_report_create_enforces_role_permissions() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let app = web_server::app(mm);

	let case_id =
		create_case(&app, &cookie_header(&admin_token.to_string()), seed.org_id)
			.await?;
	let (status, body) = create_safety_report(
		&app,
		&cookie_header(&viewer_token.to_string()),
		case_id,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

	let (status, body) = create_safety_report(
		&app,
		&cookie_header(&admin_token.to_string()),
		case_id,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(())
}
