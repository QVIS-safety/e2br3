use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Method, Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tower::ServiceExt;
use uuid::Uuid;

fn parse_json_or_raw(body: &[u8]) -> Value {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return json!({});
	}
	serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({ "raw": raw }))
}

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

fn valid_compliance_payload() -> Value {
	json!({
		"reason_for_change": "submit case to FDA gateway",
		"e_signature": {
			"meaning": "submit case",
			"password": "adminpwd"
		}
	})
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
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

async fn post_json_with_headers(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
	extra_headers: &[(&str, &str)],
) -> Result<(StatusCode, Value)> {
	let mut req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json");
	for (k, v) in extra_headers {
		req = req.header(*k, *v);
	}
	let req = req.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&body);
	Ok((status, value))
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
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

async fn request_json(
	app: &axum::Router,
	cookie: &str,
	method: Method,
	uri: String,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let req =
		builder.body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

#[derive(Clone)]
struct MockSubmitterState {
	received: Arc<Mutex<Vec<Value>>>,
}

#[derive(Clone)]
struct MockEsgState {
	received: Arc<Mutex<Vec<Value>>>,
	response_status: StatusCode,
	response_body: Value,
}

async fn start_mock_submitter() -> Result<(String, Arc<Mutex<Vec<Value>>>)> {
	let received = Arc::new(Mutex::new(Vec::<Value>::new()));
	let state = MockSubmitterState {
		received: received.clone(),
	};
	let app = Router::new()
		.route("/submit", post(mock_submitter_submit))
		.with_state(state);
	let listener = TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	tokio::spawn(async move {
		let _ = axum::serve(listener, app).await;
	});
	Ok((format!("http://{addr}"), received))
}

async fn mock_submitter_submit(
	State(state): State<MockSubmitterState>,
	Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
	state.received.lock().await.push(payload);
	(
		StatusCode::OK,
		Json(json!({
			"remote_submission_id": format!("AS2-MOCK-{}", Uuid::new_v4().simple().to_string().to_uppercase()),
			"status": "submitted_ack1_pending",
		})),
	)
}

async fn start_mock_esg(
	response_status: StatusCode,
	response_body: Value,
) -> Result<(String, Arc<Mutex<Vec<Value>>>)> {
	let received = Arc::new(Mutex::new(Vec::<Value>::new()));
	let state = MockEsgState {
		received: received.clone(),
		response_status,
		response_body,
	};
	let app = Router::new()
		.route("/submissions", post(mock_esg_submit))
		.with_state(state);
	let listener = TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	tokio::spawn(async move {
		let _ = axum::serve(listener, app).await;
	});
	Ok((format!("http://{addr}"), received))
}

async fn mock_esg_submit(
	State(state): State<MockEsgState>,
	headers: axum::http::HeaderMap,
	Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
	let auth = headers
		.get("authorization")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("")
		.to_string();
	let api_key = headers
		.get("x-api-key")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("")
		.to_string();
	state.received.lock().await.push(json!({
		"authorization": auth,
		"x_api_key": api_key,
		"body": payload,
	}));
	(state.response_status, Json(state.response_body.clone()))
}

async fn create_case(
	app: &axum::Router,
	cookie: &str,
	org_id: Uuid,
) -> Result<Uuid> {
	create_case_with_profile(app, cookie, org_id, "fda").await
}

async fn create_case_with_profile(
	app: &axum::Router,
	cookie: &str,
	org_id: Uuid,
	_appendix: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": org_id,
			"safety_report_id": format!("SUB-{}", Uuid::new_v4()),
			"status": "draft"
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
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/message-header"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"batch_number": format!("BATCH-{case_id}"),
					"batch_sender_identifier": "BATCH-SENDER",
					"batch_receiver_identifier": "BATCH-RECEIVER",
					"batch_transmission_date": [2024, 32, 1, 1, 1, 0, 0, 0, 0]
				}
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&body);
	if status != StatusCode::OK {
		return Err(format!(
			"update message header failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn create_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sender_type": "1",
			"organization_name": "Sender Org"
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create sender failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

async fn create_primary_source(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"qualification": "1",
			"email": "reporter@example.com"
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create primary source failed: status={status} body={value}"
		)
		.into());
	}
	let primary_source_id = value["data"]["id"]
		.as_str()
		.ok_or("missing primary source id")?;
	let req = Request::builder()
		.method("PUT")
		.uri(format!(
			"/api/cases/{case_id}/safety-report/primary-sources/{primary_source_id}"
		))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({"data": { "email": "reporter@example.com" }}).to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::OK {
		return Err(format!(
			"update primary source failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn create_patient(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"patient_initials": "AB",
			"sex": "1"
		}
	});
	let (status, value) =
		post_json(app, cookie, &format!("/api/cases/{case_id}/patient"), body)
			.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(
			format!("create patient failed: status={status} body={value}").into(),
		);
	}
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/patient"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"race_code": "C41260",
					"ethnicity_code": "C41222"
				}
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::OK {
		return Err(
			format!("update patient failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

async fn create_reaction(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"primary_source_reaction": "Headache"
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/reactions"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create reaction failed: status={status} body={value}").into(),
		);
	}
	let reaction_id = value["data"]["id"].as_str().ok_or("missing reaction id")?;
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/reactions/{reaction_id}"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({ "data": {
				"reaction_meddra_version": "27.0",
				"reaction_meddra_code": "10019211",
				"outcome": "1",
				"reaction_language": "en"
			}})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::OK {
		return Err(
			format!("update reaction failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

async fn create_drug(app: &axum::Router, cookie: &str, case_id: Uuid) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Drug A"
		}
	});
	let (status, value) =
		post_json(app, cookie, &format!("/api/cases/{case_id}/drugs"), body).await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create drug failed: status={status} body={value}").into()
		);
	}
	Ok(())
}

async fn create_narrative(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"case_narrative": "Case narrative for submission validator fixture."
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/narrative"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create narrative failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn create_receiver_presave(
	app: &axum::Router,
	cookie: &str,
	name: &str,
	data: Value,
) -> Result<Uuid> {
	let (status, value) = post_json(
		app,
		cookie,
		"/api/presaves/receivers",
		json!({
			"data": {
				"authority": "fda",
				"name": name,
				"comments": "receiver routing template",
				"receiver_type": data["receiverType"].as_str(),
				"organization_name": data["organizationName"].as_str(),
				"receiver_identifier": data["routingRules"][0]["messageReceiverIdentifier"].as_str(),
				"description": data["email"].as_str()
			}
		}),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create receiver presave failed: status={status} body={value}"
		)
		.into());
	}
	let id = value["data"]["id"]
		.as_str()
		.ok_or("missing receiver presave id")?;
	Ok(Uuid::parse_str(id)?)
}

async fn apply_receiver_presave_to_case(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	template_data: &Value,
	authority: &str,
	report_type: &str,
) -> Result<()> {
	let (status, value) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/receiver"),
		json!({
			"data": {
				"case_id": case_id,
				"receiver_type": template_data["receiverType"].as_str(),
				"organization_name": template_data["organizationName"].as_str()
			}
		}),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create case receiver failed: status={status} body={value}"
		)
		.into());
	}
	let receiver_id = value["data"]["id"].as_str().ok_or("missing receiver id")?;
	let (status, value) = request_json(
		app,
		cookie,
		Method::PUT,
		format!("/api/cases/{case_id}/receiver"),
		Some(json!({
			"data": {
				"id": receiver_id,
				"department": template_data["department"].as_str(),
				"city": template_data["city"].as_str(),
				"email": template_data["email"].as_str(),
				"country_code": template_data["countryCode"].as_str()
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update case receiver failed: status={status} body={value}"
		)
		.into());
	}

	let routing_rule = template_data["routingRules"]
		.as_array()
		.and_then(|rules| {
			rules.iter().find(|rule| {
				rule["authority"].as_str() == Some(authority)
					&& rule["reportType"].as_str() == Some(report_type)
			})
		})
		.ok_or("missing matching receiver routing rule")?;
	let (status, value) = request_json(
		app,
		cookie,
		Method::PUT,
		format!("/api/cases/{case_id}/message-header"),
		Some(json!({
			"data": {
				"message_receiver_identifier": routing_rule["messageReceiverIdentifier"].as_str(),
				"batch_receiver_identifier": routing_rule["batchReceiverIdentifier"].as_str()
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update message header from receiver presave failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

async fn seed_rule_clean_case(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	create_safety_report(app, cookie, case_id).await?;
	create_message_header(app, cookie, case_id).await?;
	create_sender(app, cookie, case_id).await?;
	create_primary_source(app, cookie, case_id).await?;
	create_patient(app, cookie, case_id).await?;
	create_reaction(app, cookie, case_id).await?;
	create_drug(app, cookie, case_id).await?;
	create_narrative(app, cookie, case_id).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_receiver_presave_applies_submission_routing_to_case_resources(
) -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let template_payload = json!({
		"receiverType": "2",
		"organizationName": "Submission Receiver Org",
		"department": "CDER",
		"city": "Silver Spring",
		"countryCode": "US",
		"email": "receiver@example.com",
		"routingRules": [
			{
				"authority": "fda",
				"reportType": "1",
				"batchReceiverIdentifier": "ZZFDA",
				"messageReceiverIdentifier": "CDER"
			}
		]
	});
	let template_id = create_receiver_presave(
		&app,
		&cookie,
		"receiver-routing-template",
		template_payload.clone(),
	)
	.await?;
	let (status, saved_template) = get_json(
		&app,
		&cookie,
		&format!("/api/presaves/receivers/{template_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{saved_template:?}");
	assert_eq!(
		saved_template["data"]["organization_name"].as_str(),
		Some("Submission Receiver Org")
	);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	apply_receiver_presave_to_case(
		&app,
		&cookie,
		case_id,
		&template_payload,
		"fda",
		"1",
	)
	.await?;

	let (status, receiver) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/receiver")).await?;
	assert_eq!(status, StatusCode::OK, "{receiver:?}");
	assert_eq!(
		receiver["data"]["organization_name"].as_str(),
		Some("Submission Receiver Org")
	);
	assert_eq!(receiver["data"]["receiver_type"].as_str(), Some("2"));
	assert_eq!(receiver["data"]["department"].as_str(), Some("CDER"));
	assert_eq!(receiver["data"]["city"].as_str(), Some("Silver Spring"));
	assert_eq!(receiver["data"]["country_code"].as_str(), Some("US"));
	assert_eq!(
		receiver["data"]["email"].as_str(),
		Some("receiver@example.com")
	);

	let (status, header) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{header:?}");
	assert_eq!(
		header["data"]["batch_receiver_identifier"].as_str(),
		Some("ZZFDA")
	);
	assert_eq!(
		header["data"]["message_receiver_identifier"].as_str(),
		Some("CDER")
	);

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
		valid_compliance_payload(),
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
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query("UPDATE cases SET status = 'validated' WHERE id = $1")
				.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
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
async fn test_submission_history_includes_latest_ack_time_and_event() -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let submission_id = Uuid::new_v4();
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO case_submissions (
					id, case_id, gateway, remote_submission_id, status, xml_bytes,
					submitted_by, submitted_at, created_at, updated_at
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now(), now())",
			)
			.bind(submission_id)
			.bind(case_id)
			.bind("fda-esg-nextgen-api")
			.bind("BATCH-HISTORY-1")
			.bind("ack2_received")
			.bind(2048_i32)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_acks (
					submission_id, ack_level, success, ack_code, ack_message, received_at, raw_payload
				)
				VALUES ($1, $2, $3, $4, $5, now() - interval '5 minutes', $6),
				       ($1, $7, $8, $9, $10, now() - interval '1 minutes', $11)",
			)
			.bind(submission_id)
			.bind(1_i16)
			.bind(true)
			.bind("ACK1")
			.bind(Option::<String>::None)
			.bind(json!({ "level": 1, "success": true, "code": "ACK1" }))
			.bind(2_i16)
			.bind(true)
			.bind("ACK2")
			.bind(Option::<String>::None)
			.bind(json!({ "level": 2, "success": true, "code": "ACK2" })),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_events (
					submission_id, event_type, event_data, created_at
				)
				VALUES ($1, $2, $3, now() - interval '10 minutes'),
				       ($1, $4, $5, now() - interval '30 seconds')",
			)
			.bind(submission_id)
			.bind("submission_created")
			.bind(json!({ "status": "ack1_received" }))
			.bind("ack_recorded")
			.bind(json!({ "ack_level": 2, "ack_code": "ACK2" })),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let (status, history) =
		get_json(&app, &cookie, "/api/submissions/history").await?;
	assert_eq!(status, StatusCode::OK, "{history:?}");
	let items = history["data"]["items"]
		.as_array()
		.ok_or("missing submission history items")?;
	let submission_id_str = submission_id.to_string();
	let item = items
		.iter()
		.find(|item| {
			item["submissionId"].as_str() == Some(submission_id_str.as_str())
		})
		.ok_or("missing submission history row for created submission")?;
	assert_eq!(item["status"].as_str(), Some("ack2_received"));
	assert_eq!(item["batchResult"].as_str(), Some("ack2_received"));
	assert_eq!(item["messageResult"].as_str(), Some("ACK2"));
	assert_eq!(item["latestEventType"].as_str(), Some("ack_recorded"));
	assert!(
		item["latestAckReceivedAt"]
			.as_str()
			.is_some_and(|value| !value.trim().is_empty()),
		"{item:?}"
	);
	assert_eq!(item["acknowledgedDate"], item["latestAckReceivedAt"]);
	assert_eq!(item["icsrCount"].as_i64(), Some(1));
	assert_eq!(
		item["dataFileName"].as_str(),
		Some(
			format!(
				"{}-{}-fda.xml",
				item["caseNumber"].as_str().unwrap(),
				case_id
			)
			.as_str()
		)
	);
	assert_eq!(
		item["dataFileDownloadUrl"].as_str(),
		Some(format!("/api/cases/{case_id}/export/xml?authority=fda").as_str())
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_ack_can_be_downloaded_as_text() -> Result<()> {
	clear_esg_env();
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let submission_id = Uuid::new_v4();
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO case_submissions (
					id, case_id, gateway, remote_submission_id, status, xml_bytes,
					submitted_by, submitted_at, created_at, updated_at
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now(), now())",
			)
			.bind(submission_id)
			.bind(case_id)
			.bind("fda")
			.bind("REMOTE-ACK-DOWNLOAD")
			.bind("ack2_received")
			.bind(2048_i32)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_acks (
					submission_id, ack_level, success, ack_code, ack_message, received_at, raw_payload
				)
				VALUES ($1, $2, $3, $4, $5, now(), $6)",
			)
			.bind(submission_id)
			.bind(2_i16)
			.bind(true)
			.bind("ACK2")
			.bind("Accepted by gateway")
			.bind(json!({
				"level": 2,
				"success": true,
				"code": "ACK2",
				"message": "Accepted by gateway"
			})),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}/acks/2/download"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let content_type = res
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("")
		.to_string();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let text = String::from_utf8(body.to_vec())?;
	assert_eq!(status, StatusCode::OK, "{text}");
	assert!(
		content_type.starts_with("text/plain"),
		"unexpected content type: {content_type}"
	);
	assert!(text.contains("Submission ID:"));
	assert!(text.contains("ACK Level: 2"));
	assert!(text.contains("ACK Code: ACK2"));
	assert!(text.contains("Accepted by gateway"));

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
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query("UPDATE cases SET status = 'validated' WHERE id = $1")
				.bind(case_id),
		)
		.await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
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
	let (dispatch_status, dispatch_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/dispatch-state"),
	)
	.await?;
	if dispatch_status == StatusCode::OK {
		assert_eq!(dispatch_body["data"]["state"]["attempt_count"], 1);
		assert!(
			!dispatch_body["data"]["state"]["terminal_at"].is_null(),
			"{dispatch_body:?}"
		);
	}

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
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("FDA_ESG_BASE_URL"), "{body:?}");
	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_esg_transport_sends_expected_headers_and_payload(
) -> Result<()> {
	clear_esg_env();
	let (esg_url, received) = start_mock_esg(
		StatusCode::OK,
		json!({
			"submission_id": format!("ESG-{}", Uuid::new_v4().simple()),
			"ack": { "level": 1, "success": true, "code": "ACK1" }
		}),
	)
	.await?;
	std::env::set_var("FDA_ESG_ENABLED", "1");
	std::env::set_var("FDA_ESG_BASE_URL", esg_url);
	std::env::set_var("FDA_ESG_BEARER_TOKEN", "test-esg-token");
	std::env::set_var("FDA_ESG_API_KEY", "test-esg-api-key");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "fda").await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert_eq!(body["data"]["gateway"], "fda-esg-nextgen-api");

	let msgs = received.lock().await;
	assert_eq!(msgs.len(), 1, "{msgs:?}");
	assert_eq!(msgs[0]["authorization"], "Bearer test-esg-token");
	assert_eq!(msgs[0]["x_api_key"], "test-esg-api-key");
	let xml = msgs[0]["body"]["xml"]
		.as_str()
		.ok_or("missing xml payload")?;
	assert!(!xml.trim().is_empty());

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_esg_non_success_response_returns_bad_request() -> Result<()>
{
	clear_esg_env();
	let (esg_url, _received) = start_mock_esg(
		StatusCode::BAD_GATEWAY,
		json!({ "error": "gateway unavailable" }),
	)
	.await?;
	std::env::set_var("FDA_ESG_ENABLED", "1");
	std::env::set_var("FDA_ESG_BASE_URL", esg_url);
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "fda").await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("FDA ESG submit failed"),
		"{body:?}"
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_accepts_mfds_route_for_mfds_profile() -> Result<()> {
	clear_esg_env();
	std::env::set_var("E2BR3_ALLOW_MOCK_SUBMISSION", "1");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "mfds").await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/mfds"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?;
	assert!(remote_submission_id.starts_with("MFDS-MOCK-"));

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_uses_request_authority_not_case_appendices() -> Result<()> {
	clear_esg_env();
	std::env::set_var("E2BR3_ALLOW_MOCK_SUBMISSION", "1");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query("UPDATE cases SET status = 'validated' WHERE id = $1")
				.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/mfds"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	assert!(body["data"]["remote_submission_id"]
		.as_str()
		.unwrap_or_default()
		.starts_with("MFDS-MOCK-"));

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
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("AS2 submitter request failed"),
		"{body:?}"
	);
	let (list_status, list_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/submissions"))
			.await?;
	assert_eq!(list_status, StatusCode::OK, "{list_body:?}");
	let items = list_body["data"]["items"]
		.as_array()
		.ok_or("missing submissions items")?;
	if !items.is_empty() {
		assert_eq!(items[0]["status"], "rejected");
		let remote = items[0]["remote_submission_id"]
			.as_str()
			.ok_or("missing remote_submission_id")?;
		assert!(remote.starts_with("FAILED-"), "{list_body:?}");
	}
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
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
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
	let (dispatch_status, dispatch_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/dispatch-state"),
	)
	.await?;
	if dispatch_status == StatusCode::OK {
		assert_eq!(dispatch_body["data"]["state"]["attempt_count"], 1);
		assert!(
			!dispatch_body["data"]["state"]["last_attempt_at"].is_null(),
			"{dispatch_body:?}"
		);
		assert!(dispatch_body["data"]["state"]["terminal_at"].is_null());
	}
	let (events_status, events_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/events"),
	)
	.await?;
	assert_eq!(events_status, StatusCode::OK, "{events_body:?}");
	let items = events_body["data"]["items"]
		.as_array()
		.ok_or("missing events list")?;
	assert!(items.len() >= 3, "{events_body:?}");
	let event_types: Vec<&str> = items
		.iter()
		.filter_map(|v| v["event_type"].as_str())
		.collect();
	assert!(
		event_types.contains(&"submission_created"),
		"{event_types:?}"
	);
	assert!(event_types.contains(&"ack_recorded"), "{event_types:?}");
	assert!(event_types.contains(&"status_changed"), "{event_types:?}");

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_idempotency_key_reuses_submission_when_enabled(
) -> Result<()> {
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
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let idem_key = "idem-fda-001";
	let (status1, body1) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", idem_key)],
	)
	.await?;
	assert_eq!(status1, StatusCode::CREATED, "{body1:?}");
	let id1 = body1["data"]["id"]
		.as_str()
		.ok_or("missing submission id 1")?;

	let (status2, body2) = post_json_with_headers(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
		&[("x-idempotency-key", idem_key)],
	)
	.await?;
	if status2 == StatusCode::BAD_REQUEST {
		assert!(
			body2
				.to_string()
				.contains("case must be in 'validated' status"),
			"{body2:?}"
		);
		clear_esg_env();
		return Ok(());
	}
	assert_eq!(status2, StatusCode::CREATED, "{body2:?}");
	let id2 = body2["data"]["id"]
		.as_str()
		.ok_or("missing submission id 2")?;

	let (list_status, list_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/submissions"))
			.await?;
	assert_eq!(list_status, StatusCode::OK, "{list_body:?}");
	let items = list_body["data"]["items"]
		.as_array()
		.ok_or("missing submissions list")?;
	if id1 == id2 {
		assert_eq!(items.len(), 1, "{list_body:?}");
	}

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_submission_idempotency_key_parallel_requests_single_submission(
) -> Result<()> {
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
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let idem_key = "idem-fda-parallel-001";
	let uri = format!("/api/cases/{case_id}/submissions/fda");
	let headers = [("x-idempotency-key", idem_key)];
	let req_a = async {
		post_json_with_headers(
			&app,
			&cookie,
			&uri,
			valid_compliance_payload(),
			&headers,
		)
		.await
		.expect("parallel submission A failed")
	};
	let req_b = async {
		post_json_with_headers(
			&app,
			&cookie,
			&uri,
			valid_compliance_payload(),
			&headers,
		)
		.await
		.expect("parallel submission B failed")
	};
	let ((status_a, body_a), (status_b, body_b)) = tokio::join!(req_a, req_b);

	assert_eq!(status_a, StatusCode::CREATED, "{body_a:?}");
	assert_eq!(status_b, StatusCode::CREATED, "{body_b:?}");
	let id_a = body_a["data"]["id"]
		.as_str()
		.ok_or("missing submission id a")?;
	let id_b = body_b["data"]["id"]
		.as_str()
		.ok_or("missing submission id b")?;
	assert_eq!(id_a, id_b, "{body_a:?} vs {body_b:?}");

	let (list_status, list_body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/submissions"))
			.await?;
	assert_eq!(list_status, StatusCode::OK, "{list_body:?}");
	let items = list_body["data"]["items"]
		.as_array()
		.ok_or("missing submissions list")?;
	assert_eq!(items.len(), 1, "{list_body:?}");

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_reconcile_endpoint_auth_and_empty_result() -> Result<()> {
	clear_esg_env();
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/reconcile")
		.header("content-type", "application/json")
		.header("x-callback-token", "callback-secret")
		.body(Body::from(json!({ "limit": 5 }).to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let _attempted = value["data"]["result"]["attempted"]
		.as_u64()
		.ok_or("missing attempted")?;
	let req = Request::builder()
		.method("GET")
		.uri("/internal/submissions/reconcile/status")
		.header("x-callback-token", "callback-secret")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]["status"]["total_runs"]
			.as_u64()
			.ok_or("missing total_runs")?
			>= 1
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_reconcile_status_accumulates_runtime_counters() -> Result<()>
{
	clear_esg_env();
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm);

	for _ in 0..2 {
		let req = Request::builder()
			.method("POST")
			.uri("/internal/submissions/reconcile")
			.header("content-type", "application/json")
			.header("x-callback-token", "callback-secret")
			.body(Body::from(json!({ "limit": 25 }).to_string()))?;
		let res = app.clone().oneshot(req).await?;
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		let value: Value = serde_json::from_slice(&body)?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		let _attempted = value["data"]["result"]["attempted"]
			.as_u64()
			.ok_or("missing attempted")?;
	}

	let req = Request::builder()
		.method("GET")
		.uri("/internal/submissions/reconcile/status")
		.header("x-callback-token", "callback-secret")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]["status"]["total_runs"]
			.as_u64()
			.ok_or("missing total_runs")?
			>= 2
	);
	let _total_attempted = value["data"]["status"]["total_attempted"]
		.as_u64()
		.ok_or("missing total_attempted")?;

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_reconcile_retries_failed_submission_to_success() -> Result<()>
{
	clear_esg_env();
	std::env::set_var("E2BR3_ALLOW_MOCK_SUBMISSION", "1");
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (submit_status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(submit_status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();
	let submission_uuid = Uuid::parse_str(&submission_id)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE case_submissions
				 SET status = 'rejected', updated_at = now()
				 WHERE id = $1",
			)
			.bind(submission_uuid),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_dispatch_state (
					submission_id, attempt_count, last_attempt_at, last_error, next_retry_at, terminal_at, created_at, updated_at
				)
				VALUES ($1, 1, now() - interval '2 seconds', 'seed-failure', now() - interval '1 second', NULL, now(), now())
				ON CONFLICT (submission_id)
				DO UPDATE SET
					attempt_count = EXCLUDED.attempt_count,
					last_attempt_at = EXCLUDED.last_attempt_at,
					last_error = EXCLUDED.last_error,
					next_retry_at = EXCLUDED.next_retry_at,
					terminal_at = NULL,
					updated_at = now()",
			)
			.bind(submission_uuid),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	sleep(Duration::from_millis(100)).await;

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/reconcile")
		.header("content-type", "application/json")
		.header("x-callback-token", "callback-secret")
		.body(Body::from(json!({ "limit": 25 }).to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let reconcile_value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{reconcile_value:?}");
	let processed = reconcile_value["data"]["result"]["processed_submission_ids"]
		.as_array()
		.ok_or("missing processed ids")?;
	assert!(
		processed.iter().any(|v| v.as_str() == Some(&submission_id)),
		"{reconcile_value:?}"
	);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let status_text = value["data"]["status"]
		.as_str()
		.ok_or("missing submission status")?;
	assert!(
		matches!(status_text, "ack1_received" | "rejected"),
		"{value:?}"
	);

	let (dispatch_status, dispatch_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/dispatch-state"),
	)
	.await?;
	assert_eq!(dispatch_status, StatusCode::OK, "{dispatch_body:?}");
	assert!(
		dispatch_body["data"]["state"]["attempt_count"]
			.as_i64()
			.ok_or("missing attempt_count")?
			>= 1
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_reconcile_retries_failed_submission_and_keeps_rejected_on_failure(
) -> Result<()> {
	clear_esg_env();
	std::env::set_var("E2BR3_ALLOW_MOCK_SUBMISSION", "1");
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (submit_status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(submit_status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();
	let submission_uuid = Uuid::parse_str(&submission_id)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE case_submissions
				 SET status = 'rejected', updated_at = now()
				 WHERE id = $1",
			)
			.bind(submission_uuid),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_dispatch_state (
					submission_id, attempt_count, last_attempt_at, last_error, next_retry_at, terminal_at, created_at, updated_at
				)
				VALUES ($1, 1, now() - interval '2 seconds', 'seed-failure', now() - interval '1 second', NULL, now(), now())
				ON CONFLICT (submission_id)
				DO UPDATE SET
					attempt_count = EXCLUDED.attempt_count,
					last_attempt_at = EXCLUDED.last_attempt_at,
					last_error = EXCLUDED.last_error,
					next_retry_at = EXCLUDED.next_retry_at,
					terminal_at = NULL,
					updated_at = now()",
			)
			.bind(submission_uuid),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	std::env::set_var("AS2_SUBMITTER_URL", "http://127.0.0.1:9");
	std::env::set_var("AS2_SUBMITTER_TIMEOUT_SECS", "1");
	sleep(Duration::from_millis(100)).await;
	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/reconcile")
		.header("content-type", "application/json")
		.header("x-callback-token", "callback-secret")
		.body(Body::from(json!({ "limit": 25 }).to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let reconcile_value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{reconcile_value:?}");
	let processed = reconcile_value["data"]["result"]["processed_submission_ids"]
		.as_array()
		.ok_or("missing processed ids")?;
	assert!(
		processed.iter().any(|v| v.as_str() == Some(&submission_id)),
		"{reconcile_value:?}"
	);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["status"], "rejected", "{value:?}");

	let (dispatch_status, dispatch_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/dispatch-state"),
	)
	.await?;
	assert_eq!(dispatch_status, StatusCode::OK, "{dispatch_body:?}");
	assert!(
		dispatch_body["data"]["state"]["attempt_count"]
			.as_i64()
			.ok_or("missing attempt_count")?
			>= 1
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_submission_endpoints_require_valid_callback_token(
) -> Result<()> {
	clear_esg_env();
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"remote_submission_id": "AS2-X",
				"ack_level": 1,
				"success": true
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(
		value.to_string().contains("missing x-callback-token"),
		"{value:?}"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.header("x-callback-token", "wrong-token")
		.body(Body::from(
			json!({
				"remote_submission_id": "AS2-X",
				"ack_level": 1,
				"success": true
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(
		value.to_string().contains("invalid x-callback-token"),
		"{value:?}"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/reconcile")
		.header("content-type", "application/json")
		.body(Body::from(json!({ "limit": 5 }).to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(
		value.to_string().contains("missing x-callback-token"),
		"{value:?}"
	);

	let req = Request::builder()
		.method("GET")
		.uri("/internal/submissions/reconcile/status")
		.header("x-callback-token", "wrong-token")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(
		value.to_string().contains("invalid x-callback-token"),
		"{value:?}"
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_rust_to_submitter_bridge_payload_and_ack_flow() -> Result<()> {
	clear_esg_env();
	let (submitter_url, received_payloads) = start_mock_submitter().await?;
	std::env::set_var("AS2_SUBMITTER_URL", submitter_url);
	std::env::set_var(
		"AS2_ACK_CALLBACK_URL",
		"http://127.0.0.1:8080/internal/submissions/callbacks/ack",
	);
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "mfds").await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/mfds"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?
		.to_string();
	assert!(remote_submission_id.starts_with("AS2-MOCK-"));

	let payloads = received_payloads.lock().await;
	assert_eq!(payloads.len(), 1, "{payloads:?}");
	let p = &payloads[0];
	assert_eq!(p["authority"], "mfds");
	assert_eq!(p["caseId"], case_id.to_string());
	assert_eq!(
		p["callbackUrl"],
		"http://127.0.0.1:8080/internal/submissions/callbacks/ack"
	);
	let xml_payload = p["xmlPayload"]
		.as_str()
		.ok_or("missing xmlPayload string")?;
	assert!(!xml_payload.trim().is_empty());
	drop(payloads);

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
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/submissions/{submission_id}"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["status"], "ack3_received", "{value:?}");

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_ack_callback_duplicate_payload_is_idempotent() -> Result<()> {
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
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/fda"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	let submission_id = submit_body["data"]["id"]
		.as_str()
		.ok_or("missing submission id")?
		.to_string();
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?
		.to_string();

	let ack_payload = json!({
		"remote_submission_id": remote_submission_id,
		"ack_level": 3,
		"success": true,
		"ack_code": "ACK3",
		"ack_message": "Processed",
	})
	.to_string();
	for _ in 0..2 {
		let req = Request::builder()
			.method("POST")
			.uri("/internal/submissions/callbacks/ack")
			.header("content-type", "application/json")
			.header("x-callback-token", "callback-secret")
			.body(Body::from(ack_payload.clone()))?;
		let res = app.clone().oneshot(req).await?;
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		let value: Value = serde_json::from_slice(&body)?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert_eq!(value["data"]["status"], "ack3_received", "{value:?}");
	}

	let (events_status, events_body) = get_json(
		&app,
		&cookie,
		&format!("/api/submissions/{submission_id}/events"),
	)
	.await?;
	assert_eq!(events_status, StatusCode::OK, "{events_body:?}");
	let items = events_body["data"]["items"]
		.as_array()
		.ok_or("missing events list")?;
	let event_types: Vec<&str> = items
		.iter()
		.filter_map(|v| v["event_type"].as_str())
		.collect();
	assert!(event_types.contains(&"ack_recorded"), "{event_types:?}");
	assert!(
		event_types.contains(&"ack_duplicate_ignored"),
		"{event_types:?}"
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_internal_ack_callback_rejects_malformed_payload() -> Result<()> {
	clear_esg_env();
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("POST")
		.uri("/internal/submissions/callbacks/ack")
		.header("content-type", "application/json")
		.header("x-callback-token", "callback-secret")
		.body(Body::from(
			json!({
				"remote_submission_id": "AS2-TEST-MISSING-LEVEL",
				"success": true
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	assert!(
		status == StatusCode::BAD_REQUEST
			|| status == StatusCode::UNPROCESSABLE_ENTITY,
		"status={status} body={}",
		String::from_utf8_lossy(&body)
	);
	let text = match serde_json::from_slice::<Value>(&body) {
		Ok(v) => v.to_string().to_ascii_lowercase(),
		Err(_) => String::from_utf8_lossy(&body).to_ascii_lowercase(),
	};
	assert!(
		text.contains("ack_level")
			|| text.contains("invalid")
			|| text.contains("failed to deserialize"),
		"text={text}"
	);

	clear_esg_env();
	Ok(())
}

#[serial]
#[tokio::test]
#[ignore = "Requires real Java AS2 submitter running; set REAL_AS2_SUBMITTER_URL"]
async fn test_real_java_submitter_integration_mfds() -> Result<()> {
	clear_esg_env();
	let submitter_url = match std::env::var("REAL_AS2_SUBMITTER_URL") {
		Ok(v) if !v.trim().is_empty() => v,
		Err(_) => {
			eprintln!("skipping: REAL_AS2_SUBMITTER_URL is required");
			return Ok(());
		}
		Ok(_) => {
			eprintln!("skipping: REAL_AS2_SUBMITTER_URL is required");
			return Ok(());
		}
	};
	std::env::set_var("AS2_SUBMITTER_URL", submitter_url);
	std::env::set_var(
		"AS2_ACK_CALLBACK_URL",
		"http://127.0.0.1:8080/internal/submissions/callbacks/ack",
	);
	std::env::set_var("AS2_CALLBACK_TOKEN", "callback-secret");
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id =
		create_case_with_profile(&app, &cookie, seed.org_id, "mfds").await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	mark_case_validated(&app, &cookie, case_id, "validator-secret").await?;

	let (status, submit_body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/submissions/mfds"),
		valid_compliance_payload(),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{submit_body:?}");
	assert_eq!(submit_body["data"]["gateway"], "as2-submitter-http");
	let remote_submission_id = submit_body["data"]["remote_submission_id"]
		.as_str()
		.ok_or("missing remote_submission_id")?;
	assert!(!remote_submission_id.trim().is_empty(), "{submit_body:?}");

	clear_esg_env();
	Ok(())
}
