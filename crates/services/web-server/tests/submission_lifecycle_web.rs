mod common;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

fn clear_esg_env() {
	std::env::remove_var("FDA_ESG_ENABLED");
	std::env::remove_var("FDA_ESG_BASE_URL");
	std::env::remove_var("FDA_ESG_SUBMIT_PATH");
	std::env::remove_var("FDA_ESG_BEARER_TOKEN");
	std::env::remove_var("FDA_ESG_API_KEY");
	std::env::remove_var("AS2_SUBMITTER_URL");
	std::env::remove_var("AS2_SUBMITTER_TIMEOUT_SECS");
	std::env::remove_var("AS2_ACK_CALLBACK_URL");
	std::env::remove_var("AS2_CALLBACK_TOKEN");
	std::env::remove_var("E2BR3_ALLOW_MOCK_SUBMISSION");
}

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
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn create_case(
	app: &axum::Router,
	cookie: &str,
	org_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": org_id,
			"safety_report_id": format!("SUB-{}", Uuid::new_v4()),
			"status": "draft",
			"validation_profile": "fda"
		}
	});
	let (status, value) = post_json(app, cookie, "/api/cases", body).await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create case failed: status={status} body={value}").into()
		);
	}
	let id = value["data"]["id"].as_str().ok_or("missing case id")?;
	Ok(Uuid::parse_str(id)?)
}

async fn create_safety_report(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"transmission_date": [2024, 10],
			"report_type": "1",
			"date_first_received_from_source": [2024, 10],
			"date_of_most_recent_information": [2024, 10],
			"fulfil_expedited_criteria": false
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create safety report failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn create_message_header(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"message_number": format!("MSG-{case_id}"),
			"message_sender_identifier": "SENDER01",
			"message_receiver_identifier": "CDER",
			"message_date": "20240201010101"
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/message-header"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create message header failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn mark_case_validated(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	validator_token: &str,
) -> Result<()> {
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/validator/mark-validated"))
		.header("cookie", cookie)
		.header("x-validator-token", validator_token)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::OK {
		return Err(
			format!("mark validated failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_requires_case_validated_status() -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string()
			.contains("case must be in 'validated' status"),
		"{body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_ack_out_of_order_does_not_regress_status() -> Result<()> {
	clear_esg_env();
	std::env::set_var("E2BR3_ALLOW_MOCK_SUBMISSION", "1");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();

	let (status, ack3) = post_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/acks/mock"),
		json!({ "level": 3, "success": true, "code": "ACK3" }),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{ack3:?}");
	assert_eq!(ack3["data"]["status"], "ack3_received");

	let (status, ack2) = post_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/acks/mock"),
		json!({ "level": 2, "success": true, "code": "ACK2" }),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{ack2:?}");
	assert_eq!(ack2["data"]["status"], "ack3_received");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_ack_terminal_status_does_not_change() -> Result<()> {
	clear_esg_env();
	std::env::set_var("E2BR3_ALLOW_MOCK_SUBMISSION", "1");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();

	let (status, ack4) = post_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/acks/mock"),
		json!({ "level": 4, "success": true, "code": "ACK4" }),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{ack4:?}");
	assert_eq!(ack4["data"]["status"], "ack4_received");

	let (status, ack2) = post_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/acks/mock"),
		json!({ "level": 2, "success": true, "code": "ACK2" }),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{ack2:?}");
	assert_eq!(ack2["data"]["status"], "ack4_received");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_rejects_enabled_esg_without_base_url() -> Result<()> {
	clear_esg_env();
	std::env::set_var("FDA_ESG_ENABLED", "1");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("FDA_ESG_BASE_URL"), "{body:?}");
	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_rejects_when_as2_submitter_unreachable() -> Result<()> {
	clear_esg_env();
	std::env::set_var("AS2_SUBMITTER_URL", "http://127.0.0.1:9");
	std::env::set_var("AS2_SUBMITTER_TIMEOUT_SECS", "1");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("AS2 submitter request failed"),
		"{body:?}"
	);
	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_ack_callback_updates_submission_by_remote_id() -> Result<()> {
	clear_esg_env();
	std::env::set_var("E2BR3_ALLOW_MOCK_SUBMISSION", "1");
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?
		.to_string();
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission_id")?
		.to_string();

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.header("x-callback-token", "callback-secret")
		.body(Body::from(
			json!({
				"remote_submission_id": remote_submission_id,
				"ack_level": 3,
				"success": true,
				"ack_code": "ACK3",
				"ack_message": "Processed",
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let callback_status = res.status();
	let callback_body = to_bytes(res.into_body(), usize::MAX).await?;
	let callback_value: Value = serde_json::from_slice(&callback_body)?;
	assert_eq!(callback_status, StatusCode::OK, "{callback_value:?}");
	assert_eq!(callback_value["data"]["status"], "ack3_received");

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let get_status = res.status();
	let get_body = to_bytes(res.into_body(), usize::MAX).await?;
	let get_value: Value = serde_json::from_slice(&get_body)?;
	assert_eq!(get_status, StatusCode::OK, "{get_value:?}");
	assert_eq!(get_value["data"]["status"], "ack3_received");

	clear_esg_env();
	Ok(())
}
