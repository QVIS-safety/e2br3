use crate::common::Result;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

pub(super) async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let request = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice(&body)?))
}

pub(super) async fn patch_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	patch_json_with_reason(app, cookie, uri, body, None).await
}

pub(super) async fn patch_json_with_reason(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
	reason: Option<&str>,
) -> Result<(StatusCode, Value)> {
	let mut request = Request::builder()
		.method("PATCH")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json");
	if let Some(reason) = reason {
		request = request.header("x-e2br3-reason-for-change", reason);
	}
	let request = request.body(Body::from(body.to_string()))?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice(&body)?))
}

pub(super) async fn get_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let request = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice(&body)?))
}

pub(super) async fn create_case_for_editor(
	app: &axum::Router,
	cookie: &str,
	safety_report_prefix: &str,
	_appendices: &[&str],
) -> Result<String> {
	let safety_report_id = format!("{safety_report_prefix}-{}", Uuid::new_v4());
	let (status, body) = post_json(
		app,
		cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": safety_report_id},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string())
}

pub(super) async fn repeatable_row_id(
	page_id: &str,
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<Option<String>> {
	Ok(match page_id {
		"LR" => {
			Some(create_literature_reference_fixture(app, cookie, case_id).await?)
		}
		"AE" => Some(create_reaction_contract_fixture(app, cookie, case_id).await?),
		"LB" => Some(create_test_result_fixture(app, cookie, case_id).await?),
		"DG" => Some(create_drug_fixture(app, cookie, case_id).await?),
		"DH" => Some(create_past_drug_history_fixture(app, cookie, case_id).await?),
		_ => None,
	})
}

async fn create_literature_reference_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/editor/pages/LR/rows"),
		json!({
			"authorities": ["ich"],
			"rows": {
				"literatureReference": {"sequenceNumber": 1}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["rowId"]
		.as_str()
		.ok_or("missing literature reference fixture id")?
		.to_string())
}

pub(super) async fn create_reaction_contract_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": "Fixture reaction",
				"primary_source_reaction_translation": "Fixture translation",
				"reaction_meddra_version": "27.1",
				"reaction_meddra_code": "10019211",
				"serious": false,
				"outcome": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing reaction fixture id")?
		.to_string())
}

async fn create_test_result_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/test-results"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"test_name": "Fixture test",
				"test_result_value": "0",
				"test_result_unit": "fixture"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing test result fixture id")?
		.to_string())
}

async fn create_drug_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Fixture product",
				"action_taken": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing drug fixture id")?
		.to_string())
}

async fn create_patient_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({
			"data": {
				"case_id": case_id,
				"patient_initials": "FIXTURE"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing patient fixture id")?
		.to_string())
}

async fn create_past_drug_history_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let patient_id = create_patient_fixture(app, cookie, case_id).await?;
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/patient/past-drugs"),
		json!({
			"data": {
				"patient_id": patient_id,
				"sequence_number": 1,
				"drug_name": "Fixture prior drug",
				"indication_meddra_code": "10012345"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing past drug fixture id")?
		.to_string())
}

/// Compare the batched page with canonical row routes using real persisted rows.
pub(super) async fn assert_batched_rows(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
	section: &str,
) -> Result<()> {
	let uri = format!("/api/cases/{case_id}/editor/pages/{section}");
	let (status, page) = get_json(app, cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{page}");
	assert!(page["rows"].get("rowDetails").is_none());
	let (status, expanded) =
		get_json(app, cookie, &format!("{uri}?include_rows=true")).await?;
	assert_eq!(status, StatusCode::OK, "{expanded}");
	assert_eq!(expanded["rows"]["rows"], page["rows"]["rows"]);
	let rows = page["rows"]["rows"].as_array().ok_or("missing rows")?;
	let details = expanded["rows"]["rowDetails"]
		.as_object()
		.ok_or("missing batched details")?;
	assert_eq!(details.len(), rows.len());
	for row in rows {
		let id = row["id"].as_str().ok_or("missing row id")?;
		let (status, individual) =
			get_json(app, cookie, &format!("{uri}/rows/{id}")).await?;
		assert_eq!(status, StatusCode::OK, "{individual}");
		assert_eq!(details.get(id).ok_or("missing canonical row")?, &individual);
	}
	Ok(())
}

pub(super) fn behavior_contract(page: &str, group: &str, id: &str) -> Result<Value> {
	let document = super::fields::contract_document(page)?;
	document[group]
		.as_array()
		.and_then(|checks| checks.iter().find(|check| check["id"] == id))
		.cloned()
		.ok_or_else(|| format!("missing editor {page}/{group}/{id} contract").into())
}

pub(super) const ROW_CHANGE_REASON: &str = "editor-contract:row-lifecycle";

pub(super) async fn patch_row_with_reason(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	patch_json_with_reason(app, cookie, uri, body, Some(ROW_CHANGE_REASON)).await
}
