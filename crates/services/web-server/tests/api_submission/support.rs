pub(super) use crate::common::{
	cookie_header, init_test_mm, seed_active_test_meddra_term, seed_org_with_users,
	Result,
};
pub(super) use axum::body::{to_bytes, Body};
pub(super) use axum::extract::State;
pub(super) use axum::http::{Request, StatusCode};
pub(super) use axum::routing::post;
pub(super) use axum::{Json, Router};
pub(super) use lib_auth::token::generate_web_token;
pub(super) use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
pub(super) use lib_core::model::store::set_full_context_dbx;
pub(super) use serde_json::{json, Value};
pub(super) use serial_test::serial;
pub(super) use std::sync::Arc;
pub(super) use tokio::net::TcpListener;
pub(super) use tokio::sync::Mutex;
pub(super) use tokio::time::{sleep, Duration};
pub(super) use tower::ServiceExt;
pub(super) use uuid::Uuid;

pub(super) fn parse_json_or_raw(body: &[u8]) -> Value {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return json!({});
	}
	serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({ "raw": raw }))
}

pub(super) fn clear_esg_env() {
	std::env::remove_var("FDA_ESG_ENABLED");
	std::env::remove_var("FDA_ESG_BASE_URL");
	std::env::remove_var("FDA_ESG_SUBMIT_PATH");
	std::env::remove_var("FDA_ESG_BEARER_TOKEN");
	std::env::remove_var("FDA_ESG_API_KEY");
	std::env::remove_var("AS2_SUBMITTER_URL");
	std::env::remove_var("AS2_SUBMITTER_TOKEN");
	std::env::remove_var("AS2_SUBMITTER_TIMEOUT_SECS");
	std::env::remove_var("AS2_ACK_CALLBACK_URL");
	std::env::remove_var("AS2_CALLBACK_TOKEN");
	std::env::remove_var("SUBMISSION_RECONCILE_TOKEN");
}

pub(super) fn valid_compliance_payload() -> Value {
	json!({
		"reason_for_change": "submit case to FDA gateway",
		"e_signature": {
			"meaning": "submit case",
			"password": "adminpwd"
		}
	})
}

pub(super) async fn post_json(
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

pub(super) async fn post_json_with_headers(
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

pub(super) async fn put_json(
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
	let value = parse_json_or_raw(&body);
	Ok((status, value))
}

pub(super) async fn get_json(
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

#[derive(Clone)]
pub(super) struct MockSubmitterState {
	received: Arc<Mutex<Vec<Value>>>,
}

#[derive(Clone)]
pub(super) struct MockEsgState {
	received: Arc<Mutex<Vec<Value>>>,
	response_status: StatusCode,
	response_body: Value,
}

pub(super) async fn start_mock_submitter() -> Result<(String, Arc<Mutex<Vec<Value>>>)>
{
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

pub(super) async fn mock_submitter_submit(
	State(state): State<MockSubmitterState>,
	Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
	state.received.lock().await.push(payload);
	(
		StatusCode::OK,
		Json(json!({
			"remote_submission_id": format!("AS2-MOCK-{}", Uuid::new_v4().simple().to_string().to_uppercase()),
			"ack": { "level": 1, "success": true, "code": "ACK1" },
		})),
	)
}

pub(super) async fn start_mock_esg(
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

pub(super) async fn mock_esg_submit(
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

pub(super) async fn configure_mock_esg_transport() -> Result<()> {
	let (esg_url, _received) = start_mock_esg(
		StatusCode::OK,
		json!({
			"submission_id": format!("ESG-{}", Uuid::new_v4().simple()),
			"ack": { "level": 1, "success": true, "code": "ACK1" }
		}),
	)
	.await?;
	std::env::set_var("FDA_ESG_ENABLED", "1");
	std::env::set_var("FDA_ESG_BASE_URL", esg_url);
	Ok(())
}

pub(super) async fn configure_mock_as2_transport() -> Result<()> {
	let (submitter_url, _received) = start_mock_submitter().await?;
	std::env::set_var("AS2_SUBMITTER_URL", submitter_url);
	std::env::set_var("AS2_SUBMITTER_TOKEN", "test-token");
	Ok(())
}

pub(super) async fn create_case(
	app: &axum::Router,
	cookie: &str,
	org_id: Uuid,
) -> Result<Uuid> {
	create_case_with_profile(app, cookie, org_id, "fda").await
}

pub(super) async fn create_case_with_profile(
	app: &axum::Router,
	cookie: &str,
	_org_id: Uuid,
	_appendix: &str,
) -> Result<Uuid> {
	let initial_report_id = format!("US-SENDER-{}", Uuid::new_v4().simple());
	let body = json!({
		"data": {
			"status": "draft",
			"fdaReportType": "1",
			"mfdsReportType": "1",
			"safetyReportIdentification": {
				"safetyReportId": initial_report_id
			}
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

pub(super) async fn create_safety_report(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"safety_report_id": format!("US-SENDER-{}", case_id.simple()),
			"transmission_date": "20241001000000",
				"report_type": "1",
				"date_first_received_from_source": "20241001",
				"date_of_most_recent_information": "20241001",
				"fulfil_expedited_criteria": false,
				"additional_documents_available": false,
				"worldwide_unique_id": format!("US-SENDER-{}", case_id.simple()),
				"first_sender_type": "1",
				"local_criteria_report_type": "2",
				"combination_product_report_indicator": "false",
				"other_case_identifiers_exist": null,
				"other_case_identifiers_exist_null_flavor": "NI"
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

pub(super) async fn create_message_header(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"message_number": format!("US-SENDER-{}", case_id.simple()),
			"message_sender_identifier": "SENDER01",
			"message_receiver_identifier": "CDER",
			"message_date": "20241001000000"
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
					"batch_sender_identifier": "SENDER01",
					"batch_receiver_identifier": "ZZFDA",
					"batch_transmission_date": [2024, 10, 1, 0, 0, 0, 0, 0, 0]
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

pub(super) async fn create_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let gateways = ["ich", "fda", "mfds"]
		.into_iter()
		.enumerate()
		.map(|(index, authority)| {
			json!({
				"sequenceNumber": index + 1,
				"gatewayAuthority": authority,
				"senderIdentifier": format!("TEST-{}", authority.to_ascii_uppercase()),
				"isDefaultForAuthority": true
			})
		})
		.collect::<Vec<_>>();
	let (status, value) = post_json(
		app,
		cookie,
		"/api/presaves/senders",
		json!({
			"data": { "rows": {
				"sender": {
					"senderType": "1",
					"organizationName": format!("Sender Org {case_id}")
				},
				"gateways": gateways,
				"responsiblePersons": []
			} }
		}),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create sender presave failed: status={status} body={value}"
		)
		.into());
	}
	let sender_presave_id = value["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or("missing sender presave id")?;
	let body = json!({
		"data": {
			"case_id": case_id,
			"sender_type": "1",
			"organization_name": "Sender Org",
			"department": "Drug Safety",
			"person_title": "Dr",
			"person_given_name": "Test",
			"person_family_name": "Sender",
			"street_address": "1 Test Street",
			"city": "Boston",
			"state": "MA",
			"postcode": "02108",
			"country_code": "US",
			"telephone": "+1-202-555-0100",
			"fax": "+1-202-555-0101",
			"email": "sender@example.test",
			"source_sender_presave_id": sender_presave_id
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

pub(super) async fn create_primary_source(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"qualification": "1",
				"country_code": "US",
				"email": "reporter@example.com",
				"primary_source_regulatory": "1"
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
			json!({"data": {
				"country_code": "US",
				"email": "reporter@example.com",
				"primary_source_regulatory": "1"
			}})
			.to_string(),
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

pub(super) async fn create_patient(
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
					"race_codes": ["C41260"],
					"ethnicity_code": "C41222",
					"medical_history_text": "No relevant medical history."
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

pub(super) async fn create_reaction(
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
					"reaction_meddra_version": "26.0",
					"reaction_meddra_code": "10000001",
				"outcome": "1",
				"reaction_language": "eng",
				"serious": false,
				"criteria_death_null_flavor": "NI",
				"criteria_life_threatening_null_flavor": "NI",
				"criteria_hospitalization_null_flavor": "NI",
				"criteria_disabling_null_flavor": "NI",
				"criteria_congenital_anomaly_null_flavor": "NI",
				"criteria_other_medically_important_null_flavor": "NI",
				"required_intervention": false
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

pub(super) async fn create_drug(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Drug A",
			"mpid": "MPID-TEST",
			"mpid_version": "1"
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

pub(super) async fn create_narrative(
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

pub(super) async fn seed_rule_clean_case(
	mm: &lib_core::model::ModelManager,
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	seed_active_test_meddra_term(mm).await?;
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

pub(super) async fn mark_case_validated(
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
		let (_, validation) =
			get_json(app, cookie, &format!("/api/cases/{case_id}/validation"))
				.await?;
		return Err(
			format!("mark validated failed: status={status} body={value} validation={validation}").into(),
		);
	}
	Ok(())
}
