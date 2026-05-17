use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	seed_org_with_users, system_user_id, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_USER;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn create_case(
	app: &axum::Router,
	cookie: &str,
	org_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": org_id,
			"safety_report_id": format!("SR-{}", Uuid::new_v4()),
			"status": "draft"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/cases")
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create case status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

async fn create_case_with_payload(
	app: &axum::Router,
	cookie: &str,
	payload: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri("/api/cases")
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(payload.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)
		.unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&body) }));
	Ok((status, value))
}

async fn create_safety_report(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"transmission_date": [2024, 1],
			"report_type": "1",
			"date_first_received_from_source": [2024, 1],
			"date_of_most_recent_information": [2024, 1],
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
	if status != StatusCode::CREATED {
		return Err(format!(
			"create safety report status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(())
}

async fn create_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	sender_type: &str,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sender_type": sender_type,
			"organization_name": "Test Sender Org"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/safety-report/senders"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create sender status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
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
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/cases/{case_id}/safety-report/primary-sources"
		))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create primary source status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let primary_source_id = value["data"]["id"]
		.as_str()
		.ok_or("missing primary source id")?;
	let update = json!({
		"data": {
			"email": "reporter@example.com"
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!(
			"/api/cases/{case_id}/safety-report/primary-sources/{primary_source_id}"
		))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(update.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update primary source status {} body {}",
			status,
			String::from_utf8_lossy(&body)
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
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/patient"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let update = json!({
		"data": {
			"race_code": "C41260",
			"ethnicity_code": "C41222"
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/patient"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(update.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
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
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/reactions"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let reaction_id = value["data"]["id"].as_str().ok_or("missing reaction id")?;
	let update = json!({
		"data": {
			"reaction_meddra_version": "27.0",
			"reaction_meddra_code": "10019211",
			"outcome": "1",
			"reaction_language": "en"
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/reactions/{reaction_id}"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(update.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
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
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/drugs"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create drug status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
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
			"case_narrative": "Case narrative for validator-clean fixture."
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/narrative"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create narrative status {} body {}",
			status,
			String::from_utf8_lossy(&body)
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
	create_sender(app, cookie, case_id, "1").await?;
	create_primary_source(app, cookie, case_id).await?;
	create_patient(app, cookie, case_id).await?;
	create_reaction(app, cookie, case_id).await?;
	create_drug(app, cookie, case_id).await?;
	create_narrative(app, cookie, case_id).await?;
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
			"message_receiver_identifier": "RECEIVER01",
			"message_date": "20240201010101"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/message-header"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create message header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let update = json!({
		"data": {
			"batch_number": format!("BATCH-{case_id}"),
			"batch_sender_identifier": "BATCH-SENDER",
			"batch_receiver_identifier": "BATCH-RECEIVER",
			"batch_transmission_date": [2024, 32, 1, 1, 1, 0, 0, 0, 0]
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/message-header"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(update.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update message header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(())
}

async fn update_message_header_receiver(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	batch_receiver_identifier: &str,
) -> Result<()> {
	let body = json!({
		"data": {
			"batch_receiver_identifier": batch_receiver_identifier
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/message-header"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update message header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(())
}

async fn get_validation(
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
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn update_case_status(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	status_value: &str,
) -> Result<(StatusCode, Value)> {
	let mut body = json!({
		"data": {
			"status": status_value
		}
	});
	if matches!(status_value, "submitted" | "nullified") {
		body["reason_for_change"] = json!("status transition for compliance test");
		body["e_signature"] = json!({
			"meaning": "status transition",
			"password": "adminpwd"
		});
	}
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn update_safety_report(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}/safety-report"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn get_case(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn update_admin_settings(
	app: &axum::Router,
	cookie: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri("/api/admin/settings")
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn get_workflow_config(
	app: &axum::Router,
	cookie: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri("/api/cases/workflow/config")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn get_admin_settings(
	app: &axum::Router,
	cookie: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri("/api/admin/settings")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn transition_case_workflow(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/workflow/transition"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn assign_case_workflow(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/workflow/assign"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn get_workflow_events(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/workflow/events"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

async fn validator_mark_validated(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	token: Option<&str>,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/validator/mark-validated"))
		.header("cookie", cookie);
	if let Some(token) = token {
		builder = builder.header("x-validator-token", token);
	}
	let req = builder.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

#[serial]
#[tokio::test]
async fn test_validation_defaults_to_fda_profile() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		get_validation(&app, &cookie, &format!("/api/cases/{case_id}/validation"))
			.await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["data"]["profile"], "fda");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_supports_mfds_profile() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_sender(&app, &cookie, case_id, "3").await?;

	let (status, body) = get_validation(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?profile=mfds"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["data"]["profile"], "mfds");
	assert!(
		body["data"]["issues"]
			.as_array()
			.map(|items| {
				items
					.iter()
					.any(|issue| issue["code"] == "MFDS.C.3.1.KR.1.REQUIRED")
			})
			.unwrap_or(false),
		"expected MFDS sender KR issue, body={body}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_rejects_unknown_profile() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = get_validation(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?profile=unknown"),
	)
	.await?;

	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert!(
		body.to_string().contains("invalid validation profile"),
		"unexpected body={body}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_create_case_rejects_invalid_profile() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let body = json!({
	"data": {
			"organization_id": seed.org_id,
			"safety_report_id": format!("SR-{}", Uuid::new_v4()),
			"status": "draft",
			"appendices_json": "[\"nope\"]"
		}
	});
	let (status, body) = create_case_with_payload(&app, &cookie, body).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("invalid appendix profile"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_update_case_rejects_invalid_status() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		update_case_status(&app, &cookie, case_id, "not-a-status").await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("invalid case status"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_status_transition_prevents_regression_after_submitted(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		update_case_status(&app, &cookie, case_id, "submitted").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_case_status(&app, &cookie, case_id, "draft").await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("illegal case status transition"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_nullification_code_marks_case_nullified() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"nullification_code": "1",
				"nullification_reason": "Duplicate report"
			},
			"reason_for_change": "nullify duplicate case",
			"e_signature": {
				"meaning": "nullify case",
				"password": "adminpwd"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("nullified"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_amendment_code_does_not_require_nullification_compliance() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"nullification_code": "2",
				"nullification_reason": "Follow-up correction"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("draft"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_nullification_code_requires_compliance_payload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"nullification_code": "1",
				"nullification_reason": "Duplicate report"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("reason_for_change is required"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_save_does_not_auto_transition_status_when_updating_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"safety_report_id": "UPDATED-SR-ID-001"
				}
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = serde_json::from_slice::<Value>(
		&to_bytes(res.into_body(), usize::MAX).await?,
	)?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("draft"));
	assert_eq!(
		body["data"]["safetyReportId"].as_str(),
		Some("UPDATED-SR-ID-001")
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_create_safety_report_is_idempotent_for_existing_case() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({
		"data": {
			"case_id": case_id,
			"transmission_date": [2025, 1],
			"report_type": "2",
			"date_first_received_from_source": [2025, 1],
			"date_of_most_recent_information": [2025, 1],
			"fulfil_expedited_criteria": true
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/safety-report"))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(value["data"]["id"].as_str().is_some(), "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validator_endpoint_requires_token() -> Result<()> {
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		validator_mark_validated(&app, &cookie, case_id, None).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("invalid validator token"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validator_endpoint_rejects_blocking_cases() -> Result<()> {
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		validator_mark_validated(&app, &cookie, case_id, Some("validator-secret"))
			.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("blocking issue(s) remain"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validator_endpoint_marks_validated_when_clean() -> Result<()> {
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;

	let (status, body) =
		validator_mark_validated(&app, &cookie, case_id, Some("validator-secret"))
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("validated"));

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let reason = dbx
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>,)>(
				r#"
				SELECT reason_for_change
				FROM audit_logs
				WHERE table_name = 'cases'
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'status'
				  AND changed_fields->'status'->>'new' = 'validated'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(case_id),
		)
		.await?;
	dbx.rollback_txn().await?;
	let reason = reason
		.and_then(|(v,)| v)
		.unwrap_or_default()
		.to_ascii_lowercase();
	assert!(
		reason.contains("system validation"),
		"expected system validation audit reason, got: {reason}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_save_allows_validated_to_draft_transition() -> Result<()> {
	std::env::set_var("E2BR3_VALIDATOR_TOKEN", "validator-secret");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	seed_rule_clean_case(&app, &cookie, case_id).await?;
	let (status, body) =
		validator_mark_validated(&app, &cookie, case_id, Some("validator-secret"))
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("validated"));

	let (status, body) = update_case_status(&app, &cookie, case_id, "draft").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert!(body.get("data").is_some(), "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
#[ignore = "requires DB migration/owner privileges to add 'reviewed' to case_status_valid constraint"]
async fn test_case_can_be_marked_reviewed() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		update_case_status(&app, &cookie, case_id, "reviewed").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("reviewed"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_can_be_marked_locked() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		update_case_status(&app, &cookie, case_id, "locked").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("locked"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_locked_case_rejects_content_updates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		update_case_status(&app, &cookie, case_id, "locked").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"safety_report_id": "LOCKED-EDIT-BLOCKED"
				}
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let body: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("locked cases are read-only"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_infers_mfds_profile_from_batch_receiver() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	create_message_header(&app, &cookie, case_id).await?;
	update_message_header_receiver(&app, &cookie, case_id, "ZZMFDS").await?;
	create_sender(&app, &cookie, case_id, "3").await?;

	let (status, body) =
		get_validation(&app, &cookie, &format!("/api/cases/{case_id}/validation"))
			.await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["data"]["profile"], "mfds");
	assert!(
		body["data"]["issues"]
			.as_array()
			.map(|items| {
				items
					.iter()
					.any(|issue| issue["code"] == "MFDS.C.3.1.KR.1.REQUIRED")
			})
			.unwrap_or(false),
		"expected MFDS issue from inferred profile, body={body}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validation_all_uses_appendices_profiles() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case_with_payload(
		&app,
		&cookie,
		json!({
			"data": {
				"organization_id": seed.org_id,
				"safety_report_id": format!("SR-{}", Uuid::new_v4()),
				"status": "draft",
				"appendices_json": "[\"fda\", \"mfds\", \"fda\"]"
			}
		}),
	)
	.await?
	.1["data"]["id"]
		.as_str()
		.map(Uuid::parse_str)
		.ok_or("missing data.id")??;

	create_safety_report(&app, &cookie, case_id).await?;
	create_sender(&app, &cookie, case_id, "3").await?;

	let (status, body) = get_validation(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation/all"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["profiles"], json!(["fda", "mfds"]));
	assert_eq!(
		body["data"]["reports"].as_array().map(Vec::len),
		Some(2),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_config_runtime_endpoint_returns_default_statuses(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = get_workflow_config(&app, &cookie).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert!(body["data"]["workflowEnabled"].is_boolean(), "{body:?}");
	assert_eq!(body["data"]["statuses"][0]["name"].as_str(), Some("Saved"));
	assert_eq!(
		body["data"]["statuses"][0]["editable"].as_bool(),
		Some(true)
	);
	if let Some(role) = body["data"]["statuses"][0]["allowedRoles"][0].as_str() {
		assert_eq!(role, role.to_ascii_lowercase(), "{body:?}");
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_settings_round_trips_alignment_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"timezone": "Asia/Seoul",
				"meddra_language": "English",
				"meddra_version": "28.0",
				"idf_version": "3.0",
				"company_logo": "qvis.png",
				"orientation": "Landscape",
				"data_ordering": "Primary data will appear first",
				"upload_excel_template_without_element_label": true,
				"notation": false,
				"apply_comments_on_exported_xml": true,
				"apply_sender_info_to_imported_cases": true,
				"apply_default_values_to_imported_r2_cases": false,
				"import_date_update": {
					"date_of_creation": true,
					"most_recent_info_date": true,
					"report_first_received_date": false
				},
				"appendices": ["ICH", "FDA", "MFDS"],
				"case_number_setting": "AE Row No.",
				"case_number_identifier": "SAFETY",
				"case_number_padding": 7,
				"case_number_sequence_condition": "Per sender",
				"case_number_format_fields": ["AE Row No.", "Country Code"],
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["pvs"],
							"due_days": 0,
							"description": "Default state"
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let data = &body;
	assert_eq!(data["meddra_version"].as_str(), Some("28.0"));
	assert_eq!(data["idf_version"].as_str(), Some("3.0"));
	assert_eq!(data["orientation"].as_str(), Some("Landscape"));
	assert_eq!(data["apply_comments_on_exported_xml"].as_bool(), Some(true));
	assert_eq!(
		data["import_date_update"]["most_recent_info_date"].as_bool(),
		Some(true)
	);
	assert_eq!(
		data["case_number_format_fields"][1].as_str(),
		Some("Country Code")
	);
	assert_eq!(
		data["workflow"]["statuses"][0]["due_days"].as_i64(),
		Some(0)
	);

	let (status, body) = get_admin_settings(&app, &cookie).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let data = &body;
	assert_eq!(data["company_logo"].as_str(), Some("qvis.png"));
	assert_eq!(
		data["case_number_sequence_condition"].as_str(),
		Some("Per sender")
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_number_settings_generate_c11_and_c181() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let identifier = format!("SAFETY{}", Uuid::new_v4().simple());
	let first_case_number = format!("{identifier}001");
	let second_case_number = format!("{identifier}002");

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"case_number_identifier": identifier,
				"case_number_padding": 3,
				"case_number_format_fields": ["AE Row No."]
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, first) = create_case_with_payload(
		&app,
		&cookie,
		json!({
			"data": {
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{first:?}");
	let first_id =
		Uuid::parse_str(first["data"]["id"].as_str().ok_or("missing first id")?)?;
	assert_eq!(
		first["data"]["safety_report_id"].as_str(),
		Some(first_case_number.as_str())
	);
	assert_eq!(first["data"]["version"].as_i64(), Some(1));

	let (status, second) = create_case_with_payload(
		&app,
		&cookie,
		json!({
			"data": {
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{second:?}");
	assert_eq!(
		second["data"]["safety_report_id"].as_str(),
		Some(second_case_number.as_str())
	);
	assert_eq!(second["data"]["version"].as_i64(), Some(1));

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{first_id}/safety-report"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let safety_report: Value = serde_json::from_slice(&body)?;
	assert_eq!(
		status,
		StatusCode::OK,
		"safety report status {} body {}",
		status,
		String::from_utf8_lossy(&body)
	);
	assert_eq!(
		safety_report["data"]["worldwide_unique_id"].as_str(),
		Some(first_case_number.as_str())
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_negative_due_days() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["pvs"],
							"due_days": -1
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("due_days"), "{body:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_unknown_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["not_a_real_role"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("unknown role 'not_a_real_role'"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_reject_system_admin_role() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["system_admin"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("unknown role 'system_admin'"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_settings_allow_empty_roles_as_unrestricted() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": [],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["workflow"]["statuses"][0]["allowed_roles"], json!([]));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_transition_updates_case_and_persists_event() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"allowed_roles": ["PVS", "PVM"]
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"description": "Pending internal review",
							"allowed_roles": ["PVM"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = transition_case_workflow(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "PVM",
				"comment": "Ready for review",
				"due_at": "2026-04-20T09:00:00Z"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(
		body["data"]["workflow_status"].as_str(),
		Some("To be reviewed")
	);
	assert_eq!(body["data"]["workflow_assigned_role"].as_str(), Some("pvm"));

	let (status, events) = get_workflow_events(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{events:?}");
	assert_eq!(events["data"].as_array().map(|rows| rows.len()), Some(1));
	assert_eq!(events["data"][0]["fromStatus"].as_str(), Some("Saved"));
	assert_eq!(
		events["data"][0]["toStatus"].as_str(),
		Some("To be reviewed")
	);
	assert_eq!(events["data"][0]["targetRole"].as_str(), Some("pvm"));
	assert_eq!(
		events["data"][0]["comment"].as_str(),
		Some("Ready for review")
	);
	assert_eq!(events["data"][0]["usedAdminOverride"].as_bool(), Some(true));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_assignment_updates_owner_without_changing_status(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Initial state",
							"allowed_roles": ["user"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) = assign_case_workflow(
		&app,
		&admin_cookie,
		case_id,
		json!({
			"data": {
				"target_role": "user",
				"target_user_id": seed.user.id,
				"comment": "Assign authoring owner",
				"due_at": "2026-04-21T09:00:00Z"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["workflow_status"].as_str(), Some("Saved"));
	assert_eq!(
		body["data"]["workflow_assigned_role"].as_str(),
		Some("user")
	);
	assert_eq!(body["data"]["can_act_on_workflow"].as_bool(), Some(true));
	assert_eq!(
		body["data"]["workflow_block_reason"].as_str(),
		Some("workflow_admin_override_allowed")
	);
	let assigned_user_id = seed.user.id.to_string();
	assert_eq!(
		body["data"]["workflow_assigned_user_id"].as_str(),
		Some(assigned_user_id.as_str())
	);

	let (status, events) = get_workflow_events(&app, &admin_cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{events:?}");
	assert_eq!(events["data"].as_array().map(|rows| rows.len()), Some(1));
	assert_eq!(events["data"][0]["fromStatus"].as_str(), Some("Saved"));
	assert_eq!(events["data"][0]["toStatus"].as_str(), Some("Saved"));
	assert_eq!(
		events["data"][0]["comment"].as_str(),
		Some("Assign authoring owner")
	);
	assert_eq!(events["data"][0]["usedAdminOverride"].as_bool(), Some(true));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_editable_workflow_status_blocks_subresource_write() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"allowed_roles": ["PVS"]
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"description": "Pending internal review",
							"allowed_roles": ["PVM"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) = transition_case_workflow(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "PVM",
				"comment": "Hand off"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"report_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string()
			.contains("workflow status 'To be reviewed' is read-only"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_qced_case_blocks_content_updates_even_when_workflow_saved_is_editable(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"allowed_roles": ["PVS", "PVM"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) =
		update_case_status(&app, &cookie, case_id, "reviewed").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"report_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("QCed cases are read-only"),
		"{body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validated_case_blocks_content_updates_even_when_workflow_saved_is_editable(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"allowed_roles": ["PVS", "PVM"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) =
		update_case_status(&app, &cookie, case_id, "validated").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"report_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("QCed cases are read-only"),
		"{body:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_transition_rejects_user_outside_current_step_role(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let user_cookie = cookie_header(&user_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Owned by PVS only",
							"allowed_roles": ["PVS"]
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"description": "Pending internal review",
							"allowed_roles": ["PVM"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) = transition_case_workflow(
		&app,
		&user_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "PVM",
				"comment": "User should not own Saved"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string()
			.contains("workflow status 'Saved' is assigned to a different role"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_transition_rejects_user_outside_current_assignee(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let other_user =
		insert_user(&mm, seed.org_id, ROLE_USER, system_user_id(), None).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let other_user_token =
		generate_web_token(&other_user.email, other_user.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let other_user_cookie = cookie_header(&other_user_token.to_string());
	let user_cookie = cookie_header(&user_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Initial state",
							"allowed_roles": ["user"]
						},
						{
							"name": "Assigned",
							"editable": true,
							"description": "Owned by one assigned user",
							"allowed_roles": ["user"]
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"description": "Pending internal review",
							"allowed_roles": ["manager"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) = transition_case_workflow(
		&app,
		&admin_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "Assigned",
				"target_role": "user",
				"target_user_id": seed.user.id,
				"comment": "Assign to specific user"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = transition_case_workflow(
		&app,
		&other_user_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "manager",
				"comment": "Wrong user should be blocked"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string()
			.contains("workflow status 'Assigned' is assigned to a different user"),
		"{body:?}"
	);

	let (status, body) = transition_case_workflow(
		&app,
		&user_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "manager",
				"comment": "Assigned user can transition"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_admin_override_is_allowed_and_audited() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Owned by user role",
							"allowed_roles": ["user"]
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"description": "Pending manager review",
							"allowed_roles": ["manager"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) = transition_case_workflow(
		&app,
		&admin_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "manager",
				"comment": "Admin override handoff"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["can_act_on_workflow"].as_bool(), Some(true));
	assert_eq!(
		body["data"]["workflow_block_reason"].as_str(),
		Some("workflow_admin_override_allowed")
	);

	let (status, events) = get_workflow_events(&app, &admin_cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{events:?}");
	assert_eq!(events["data"][0]["usedAdminOverride"].as_bool(), Some(true));
	assert_eq!(
		events["data"][0]["actorRoleId"].as_str(),
		Some("sponsor_admin_cro")
	);
	assert!(
		events["data"][0]["overrideReason"]
			.as_str()
			.unwrap_or_default()
			.contains("audited admin policy"),
		"{events:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_locked_case_blocks_workflow_transition_even_for_admin_override(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["user"]
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"allowed_roles": ["manager"]
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) =
		update_case_status(&app, &admin_cookie, case_id, "locked").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = transition_case_workflow(
		&app,
		&admin_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "manager"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("locked cases are read-only"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_read_returns_separate_qc_and_lock_axes() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["qc_state"].as_str(), Some("Pending"));
	assert_eq!(body["data"]["is_locked"].as_bool(), Some(false));

	let (status, body) =
		update_case_status(&app, &cookie, case_id, "reviewed").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["qc_state"].as_str(), Some("QCed"));
	assert_eq!(body["data"]["is_locked"].as_bool(), Some(false));

	let (status, body) =
		update_case_status(&app, &cookie, case_id, "locked").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["is_locked"].as_bool(), Some(true));
	Ok(())
}
