pub(super) use crate::common::{
	cookie_header, init_test_mm, seed_org_with_users, Result,
};
pub(super) use axum::body::{to_bytes, Body};
pub(super) use axum::http::{Request, StatusCode};
pub(super) use axum::Router;
pub(super) use lib_auth::token::generate_web_token;
pub(super) use serde_json::{json, Value};
pub(super) use serial_test::serial;
pub(super) use tower::ServiceExt;
pub(super) use uuid::Uuid;

pub(super) fn extract_id(body: &[u8]) -> Result<Uuid> {
	let value: Value = serde_json::from_slice(body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

pub(super) async fn post_json(
	app: &Router,
	cookie: &str,
	uri: String,
	body: Value,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

pub(super) async fn put_json_with_audit_reason(
	app: &Router,
	cookie: &str,
	uri: String,
	body: Value,
	reason: &str,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.header("x-e2br3-reason-for-change", reason)
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

pub(super) async fn get_json(
	app: &Router,
	cookie: &str,
	uri: String,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

pub(super) async fn delete_json(
	app: &Router,
	cookie: &str,
	uri: String,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("DELETE")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

pub(super) async fn request_json_with_reason(
	app: &Router,
	cookie: &str,
	method: &str,
	uri: String,
	body: Option<Value>,
	reason: &str,
) -> Result<(StatusCode, Value)> {
	let mut request = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if method != "GET" {
		request = request.header("x-e2br3-reason-for-change", reason);
	}
	let body = match body {
		Some(value) => {
			request = request.header("content-type", "application/json");
			Body::from(value.to_string())
		}
		None => Body::empty(),
	};
	let response = app.clone().oneshot(request.body(body)?).await?;
	let status = response.status();
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let value = if bytes.is_empty() {
		Value::Null
	} else {
		serde_json::from_slice(&bytes)?
	};
	Ok((status, value))
}

pub(super) async fn create_case(
	app: &Router,
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
	let (status, body) =
		post_json(app, cookie, "/api/cases".to_string(), body).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create case status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

pub(super) async fn create_patient(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"patient_initials": "AB",
			"sex": "1"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

pub(super) async fn create_patient_with_narrative_preview_values(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"patient_initials": "CD",
			"age_at_time_of_onset": 30,
			"sex": "2"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/patient"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create patient for narrative preview status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

pub(super) async fn create_narrative(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"case_narrative": "test narrative",
			"additional_information": "test sponsor information"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/narrative"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create narrative status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

pub(super) async fn create_receiver(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"receiver_type": "1",
			"organization_name": "Receiver Org A"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/receiver"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create receiver status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

pub(super) async fn create_safety_report(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
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
	let (status, body) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/safety-report"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create safety report status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

pub(super) async fn create_drug(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Test Drug"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/drugs"), body).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create drug status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

pub(super) async fn create_reaction(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"primary_source_reaction": "Headache"
		}
	});
	let (status, body) =
		post_json(app, cookie, format!("/api/cases/{case_id}/reactions"), body)
			.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create reaction status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}

pub(super) async fn create_reaction_assessment(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	drug_id: Uuid,
	reaction_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"drug_id": drug_id,
			"reaction_id": reaction_id
		}
	});
	let (status, body) = post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create reaction assessment status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	extract_id(&body)
}
