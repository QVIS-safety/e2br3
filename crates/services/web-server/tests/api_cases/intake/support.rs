pub(super) use crate::common::{
	cookie_header, init_test_mm, seed_org_with_users, Result,
};
pub(super) use axum::body::{to_bytes, Body};
pub(super) use axum::http::{Request, StatusCode};
pub(super) use lib_auth::token::generate_web_token;
pub(super) use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
pub(super) use lib_core::model::store::set_full_context_dbx;
pub(super) use serde_json::{json, Value};
pub(super) use serial_test::serial;
pub(super) use time::Date;
pub(super) use tower::ServiceExt;
pub(super) use uuid::Uuid;

pub(super) fn parse_json_or_raw(body: &[u8]) -> Value {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return json!({});
	}
	serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({ "raw": raw }))
}

pub(super) async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: serde_json::Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("content-type", "application/json")
		.header("cookie", cookie)
		.body(Body::from(body.to_string()))?;
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
	body: serde_json::Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("content-type", "application/json")
		.header("cookie", cookie)
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

pub(super) async fn get_bytes(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Vec<u8>)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, body.to_vec()))
}

pub(super) async fn create_narrative(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/narrative"),
		json!({
			"data": {
				"case_id": case_id,
				"case_narrative": "test narrative",
				"additional_information": "test sponsor information"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(())
}

pub(super) async fn create_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
	authority: &str,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		"/api/presaves/senders",
		json!({
			"data": { "rows": {
				"sender": {
					"senderType": "1",
					"organizationName": "Test Sender"
				},
				"gateways": [{
					"sequenceNumber": 1,
					"gatewayAuthority": authority,
					"senderIdentifier": "TEST-SENDER",
					"isDefaultForAuthority": true
				}],
				"responsiblePersons": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let sender_presave_id = body["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or("missing sender presave id")?;
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
		json!({
			"data": {
				"case_id": case_id,
				"sender_type": "1",
				"organization_name": "Test Sender",
				"source_sender_presave_id": sender_presave_id
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(())
}

pub(super) async fn create_unlinked_sender(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
		json!({
			"data": {
				"case_id": case_id,
				"sender_type": "1",
				"organization_name": "Test Sender"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(())
}

pub(super) async fn set_reaction_outcome(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) =
		get_json(app, cookie, &format!("/api/cases/{case_id}/reactions")).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let reaction_id = body["data"][0]["id"]
		.as_str()
		.ok_or("missing intake reaction id")?;
	let (status, body) = put_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		json!({ "data": { "outcome": "1" } }),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	Ok(())
}

pub(super) async fn create_suspect_drug(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Test Drug"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	Ok(())
}

pub(super) fn extract_case_id(body: &Value) -> Result<String> {
	Ok(body["data"]["case_id"]
		.as_str()
		.ok_or("missing case_id")?
		.to_string())
}

pub(super) fn intake_basis(
	safety_report_id: &str,
	day_of_year: u32,
	report_type: &str,
) -> Value {
	let date = Date::from_ordinal_date(2024, day_of_year as u16)
		.expect("valid test ordinal date");
	let date = format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	);
	json!({
		"authority": "ich",
		"safety_report_id": safety_report_id,
		"date_of_most_recent_information": date,
		"report_type": report_type,
		"patient_initials": intake_patient_initials(safety_report_id),
		"age_d2_2a": "41",
		"sex_d5": "2",
		"dg_prd_key": format!("DG-{}", intake_patient_initials(safety_report_id)),
		"reaction_meddra_version": "27.0",
		"reaction_meddra_code": "10019211",
		"ae_start_date": date.clone()
	})
}

pub(super) fn intake_patient_initials(safety_report_id: &str) -> String {
	let suffix: String = safety_report_id
		.chars()
		.filter(|c| c.is_ascii_alphanumeric())
		.rev()
		.take(6)
		.collect::<Vec<_>>()
		.into_iter()
		.rev()
		.collect();
	format!("P{}", suffix.to_ascii_uppercase())
}

pub(super) fn follow_up_pages(authority: &str) -> Value {
	let direct = || json!({ "authorities": [authority], "rows": {} });
	json!({
		"ci": direct(),
		"rp": direct(),
		"sd": direct(),
		"si": direct(),
		"dm": direct(),
		"nr": direct(),
		"lr": [],
		"dh": [],
		"ae": [],
		"lb": [],
		"dg": []
	})
}

pub(super) fn intake_data(
	safety_report_id: &str,
	day_of_year: u32,
	report_type: &str,
	extra: Value,
) -> Value {
	let mut base = intake_basis(safety_report_id, day_of_year, report_type);
	let base_map = base
		.as_object_mut()
		.expect("intake basis should be a JSON object");
	let extra_map = extra
		.as_object()
		.expect("intake extra should be a JSON object");
	for (key, value) in extra_map {
		base_map.insert(key.clone(), value.clone());
	}
	base
}
