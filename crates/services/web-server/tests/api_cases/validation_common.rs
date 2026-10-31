use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn parse_json_or_raw(body: &[u8]) -> Result<Value> {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return Ok(Value::Null);
	}
	Ok(serde_json::from_slice::<Value>(body)
		.unwrap_or_else(|_| json!({ "raw": raw })))
}

pub struct ValidationCtx {
	pub app: Router,
	pub cookie: String,
	pub case_id: Uuid,
	pub mm: ModelManager,
}

pub async fn setup_case() -> Result<ValidationCtx> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	Ok(ValidationCtx {
		app,
		cookie,
		case_id,
		mm,
	})
}

pub async fn post_json(
	app: &Router,
	cookie: &str,
	uri: String,
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
	let value = parse_json_or_raw(&body)?;
	Ok((status, value))
}

pub async fn put_json(
	app: &Router,
	cookie: &str,
	uri: String,
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
	let value = parse_json_or_raw(&body)?;
	Ok((status, value))
}

pub async fn get_json(
	app: &Router,
	cookie: &str,
	uri: String,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, parse_json_or_raw(&body)?))
}

pub async fn create_case(app: &Router, cookie: &str, _org_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"safetyReportIdentification": {
				"safetyReportId": format!("SR-VAL-{}", Uuid::new_v4())
			},
			"status": "draft"
		}
	});
	let (status, value) =
		post_json(app, cookie, "/api/cases".to_string(), body).await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create case failed: status={status} body={value}").into()
		);
	}
	extract_id(&value)
}

pub async fn create_safety_report(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	fulfil_expedited_criteria: bool,
) -> Result<Uuid> {
	create_safety_report_with(app, cookie, case_id, "1", fulfil_expedited_criteria)
		.await
}

pub async fn create_safety_report_with(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	report_type: &str,
	fulfil_expedited_criteria: bool,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"transmission_date": "20240101000000",
			"report_type": report_type,
			"date_first_received_from_source": "20240101",
			"date_of_most_recent_information": "20240101",
			"fulfil_expedited_criteria": fulfil_expedited_criteria
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create safety-report failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_message_header(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	batch_receiver_identifier: Option<&str>,
) -> Result<Uuid> {
	create_message_header_with_receiver(
		app,
		cookie,
		case_id,
		batch_receiver_identifier,
		"RECEIVER01",
	)
	.await
}

pub async fn create_message_header_with_receiver(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	batch_receiver_identifier: Option<&str>,
	message_receiver_identifier: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"message_number": format!("MSG-{case_id}"),
			"message_sender_identifier": "SENDER01",
			"message_receiver_identifier": message_receiver_identifier,
			"message_date": "20240201010101",
			"batch_receiver_identifier": batch_receiver_identifier
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/message-header"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create message-header failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_sender(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sender_type: &str,
	organization_name: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sender_type": sender_type,
			"organization_name": organization_name
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report/senders"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create sender failed: status={status} body={value}").into(),
		);
	}
	extract_id(&value)
}

pub async fn create_primary_source(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	qualification: Option<&str>,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"qualification": qualification
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create primary-source failed: status={status} body={value}"
		)
		.into());
	}
	extract_id(&value)
}

pub async fn create_reaction(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	primary_source_reaction: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"primary_source_reaction": primary_source_reaction
		}
	});
	let (status, value) =
		post_json(app, cookie, format!("/api/cases/{case_id}/reactions"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create reaction failed: status={status} body={value}").into(),
		);
	}
	extract_id(&value)
}

pub async fn create_test_result(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	test_name: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"test_name": test_name
		}
	});
	let (status, value) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/test-results"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create test-result failed: status={status} body={value}"
		)
		.into());
	}
	let id = extract_id(&value)?;
	let (status, value) = put_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/test-results/{id}"),
		json!({"data": { "result_unstructured": "Baseline result" }}),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"prime test-result failed: status={status} body={value}"
		)
		.into());
	}
	Ok(id)
}

pub async fn create_drug(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	sequence_number: i32,
	drug_characterization: &str,
	medicinal_product: &str,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": sequence_number,
			"drug_characterization": drug_characterization,
			"medicinal_product": medicinal_product
		}
	});
	let (status, value) =
		post_json(app, cookie, format!("/api/cases/{case_id}/drugs"), body).await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create drug failed: status={status} body={value}").into()
		);
	}
	extract_id(&value)
}

fn extract_id(value: &Value) -> Result<Uuid> {
	let id = value["data"]["id"]
		.as_str()
		.ok_or("missing data.id in response body")?;
	Ok(Uuid::parse_str(id)?)
}
