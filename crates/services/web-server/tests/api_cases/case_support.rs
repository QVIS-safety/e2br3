pub(super) use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_active_test_meddra_term,
	seed_org_with_all_roles, seed_org_with_users, system_user_id, Result,
};
pub(super) use axum::body::{to_bytes, Body};
pub(super) use axum::http::{Request, StatusCode};
pub(super) use lib_auth::token::generate_web_token;
pub(super) use lib_core::ctx::{ROLE_SPONSOR_ADMIN_CRO, ROLE_USER};
pub(super) use lib_core::model::store::set_full_context_dbx;
pub(super) use serde_json::{json, Value};
pub(super) use serial_test::serial;
pub(super) use tower::ServiceExt;
pub(super) use uuid::Uuid;

pub(super) async fn create_case(
	app: &axum::Router,
	cookie: &str,
	_org_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"safetyReportIdentification": {
				"safetyReportId": format!("SR-{}", Uuid::new_v4())
			},
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

pub(super) async fn editor_patch_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let request = Request::builder()
		.method("PATCH")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice(&body)?))
}

pub(super) async fn create_case_with_payload(
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

pub(super) async fn create_safety_report(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"safety_report_id": format!("US-SENDER-{}", case_id.simple()),
			"transmission_date": "20240101000000",
			"report_type": "1",
			"date_first_received_from_source": "20240101",
				"date_of_most_recent_information": "20240101",
				"fulfil_expedited_criteria": false,
				"combination_product_report_indicator": "false",
				"other_case_identifiers_exist": null,
				"other_case_identifiers_exist_null_flavor": "NI"
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

pub(super) async fn create_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	sender_type: &str,
) -> Result<()> {
	create_sender_with_kr1(app, cookie, case_id, sender_type, None).await
}

pub(super) async fn create_sender_with_kr1(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	sender_type: &str,
	health_professional_type_kr1: Option<&str>,
) -> Result<()> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sender_type": sender_type,
			"health_professional_type_kr1": health_professional_type_kr1,
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
				"country_code": "KR",
				"email": "reporter@example.com",
				"primary_source_regulatory": "1"
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
				"qualification": "1",
				"country_code": "KR",
				"email": "reporter@example.com",
				"primary_source_regulatory": "1"
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

pub(super) async fn force_validated_case_fixture(
	mm: &lib_core::model::ModelManager,
	user_id: Uuid,
	org_id: Uuid,
	case_id: Uuid,
) -> Result<()> {
	mm.dbx().begin_txn().await?;
	if let Err(err) =
		set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SPONSOR_ADMIN_CRO).await
	{
		let _ = mm.dbx().rollback_txn().await;
		return Err(err.into());
	}
	mm.dbx()
		.execute(
			sqlx::query("UPDATE cases SET status = 'validated' WHERE id = $1")
				.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
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
			"race_codes": ["C41260"],
			"ethnicity_code": "C41222",
			"medical_history_text": "No relevant medical history."
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
				"reaction_meddra_version": "26.0",
				"reaction_meddra_code": "10000001",
				"outcome": "1",
				"reaction_language": "eng",
				"criteria_death_null_flavor": "NI",
				"criteria_life_threatening_null_flavor": "NI",
				"criteria_hospitalization_null_flavor": "NI",
				"criteria_disabling_null_flavor": "NI",
				"criteria_congenital_anomaly_null_flavor": "NI",
				"criteria_other_medically_important_null_flavor": "NI"
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
			"mpid": "MPID-TEST"
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

pub(super) async fn create_narrative(
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

pub(super) async fn seed_rule_clean_case(
	mm: &lib_core::model::ModelManager,
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	seed_active_test_meddra_term(mm).await?;
	create_safety_report(app, cookie, case_id).await?;
	let (status, body) = update_safety_report(
		app,
		cookie,
		case_id,
		json!({
			"data": {
				"additional_documents_available": false,
				"worldwide_unique_id": format!("US-{}", case_id.simple()),
				"first_sender_type": "1"
			}
		}),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update clean safety report status {status} body {body}"
		)
		.into());
	}
	create_message_header(app, cookie, case_id).await?;
	create_sender(app, cookie, case_id, "1").await?;
	create_primary_source(app, cookie, case_id).await?;
	create_patient(app, cookie, case_id).await?;
	create_reaction(app, cookie, case_id).await?;
	create_drug(app, cookie, case_id).await?;
	create_narrative(app, cookie, case_id).await?;
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
			"message_receiver_identifier": "RECEIVER01",
			"message_date": "20240101000000"
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
			"batch_transmission_date": [2024, 1, 1, 0, 0, 0, 0, 0, 0]
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

pub(super) async fn update_message_header_receiver(
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

pub(super) async fn update_message_header_message_receiver(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	message_receiver_identifier: &str,
) -> Result<()> {
	let body = json!({
		"data": {
			"message_receiver_identifier": message_receiver_identifier
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

pub(super) async fn get_validation(
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
	let value = serde_json::from_slice::<Value>(&body)
		.unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&body) }));
	Ok((status, value))
}

pub(super) async fn update_case_status(
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

pub(super) async fn toggle_case_action(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	action: &str,
) -> Result<(StatusCode, Value)> {
	let (current_status, current) = get_case(app, cookie, case_id).await?;
	if current_status != StatusCode::OK {
		return Ok((current_status, current));
	}
	let expected_status = current["data"]["status"]
		.as_str()
		.ok_or("case response is missing status")?;
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/{action}/toggle"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({"data":{"expected_status":expected_status}}).to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&body)?;
	Ok((status, value))
}

pub(super) async fn update_safety_report(
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

pub(super) async fn get_case(
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

pub(super) async fn update_admin_settings(
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

pub(super) async fn get_workflow_config(
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

pub(super) async fn transition_case_workflow(
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

pub(super) async fn assign_case_workflow(
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

pub(super) async fn get_workflow_events(
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

pub(super) async fn validator_mark_validated(
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
