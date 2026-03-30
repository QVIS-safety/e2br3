use crate::common::Result;
use crate::persist_workflow::{create_case, request_json, PersistTestCtx};
use axum::http::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

pub struct FieldCase {
	pub canonical_id: &'static str,
	pub endpoint: &'static str,
}

pub async fn create_case_with_field(
	ctx: &PersistTestCtx,
	canonical_id: &'static str,
	field_name: &'static str,
	field_value: Value,
) -> Result<(FieldCase, Uuid)> {
	let case_id = create_case(ctx).await?;
	let payload = json!({ "data": { field_name: field_value } });
	put_ok(
		ctx,
		FieldCase {
			canonical_id,
			endpoint: "/api/cases/{id}",
		},
		format!("/api/cases/{case_id}"),
		payload,
	)
	.await?;
	Ok((
		FieldCase {
			canonical_id,
			endpoint: "/api/cases/{id}",
		},
		case_id,
	))
}

pub async fn post_created(
	ctx: &PersistTestCtx,
	field: FieldCase,
	uri: String,
	body: Value,
) -> Result<Value> {
	let (status, value) =
		request_json(&ctx.app, &ctx.cookie, "POST", uri.clone(), Some(body)).await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"{} create via {} failed: status={status} uri={uri} body={value}",
			field.canonical_id, field.endpoint
		)
		.into());
	}
	Ok(value)
}

pub async fn put_ok(
	ctx: &PersistTestCtx,
	field: FieldCase,
	uri: String,
	body: Value,
) -> Result<Value> {
	let (status, value) =
		request_json(&ctx.app, &ctx.cookie, "PUT", uri.clone(), Some(body)).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"{} update via {} failed: status={status} uri={uri} body={value}",
			field.canonical_id, field.endpoint
		)
		.into());
	}
	Ok(value)
}

pub async fn get_ok(
	ctx: &PersistTestCtx,
	field: FieldCase,
	uri: String,
) -> Result<Value> {
	let (status, value) =
		request_json(&ctx.app, &ctx.cookie, "GET", uri.clone(), None).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"{} read via {} failed: status={status} uri={uri} body={value}",
			field.canonical_id, field.endpoint
		)
		.into());
	}
	Ok(value)
}

pub fn extract_id(value: &Value) -> Result<Uuid> {
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

pub fn assert_str(body: &Value, key: &str, expected: &str) {
	assert_eq!(
		body["data"][key].as_str(),
		Some(expected),
		"field {key} mismatch in {body}"
	);
}

pub fn assert_bool(body: &Value, key: &str, expected: bool) {
	assert_eq!(
		body["data"][key].as_bool(),
		Some(expected),
		"field {key} mismatch in {body}"
	);
}

pub fn assert_i64(body: &Value, key: &str, expected: i64) {
	assert_eq!(
		body["data"][key].as_i64(),
		Some(expected),
		"field {key} mismatch in {body}"
	);
}

pub fn assert_f64(body: &Value, key: &str, expected: f64) {
	let actual = if let Some(value) = body["data"][key].as_f64() {
		value
	} else if let Some(value) = body["data"][key].as_str() {
		value
			.parse::<f64>()
			.unwrap_or_else(|_| panic!("expected numeric string {key} in {body}"))
	} else {
		panic!("expected numeric {key} in {body}");
	};
	assert!(
		(actual - expected).abs() < 0.000_001,
		"field {key} mismatch: expected {expected}, got {actual} in {body}"
	);
}

pub fn assert_null(body: &Value, key: &str) {
	assert!(
		body["data"][key].is_null(),
		"expected {key} to be null in {body}"
	);
}

pub fn assert_date_tuple(body: &Value, key: &str, expected: &[i64]) {
	let actual = body["data"][key]
		.as_array()
		.unwrap_or_else(|| panic!("expected {key} date array in {body}"));
	assert_eq!(
		actual.len(),
		expected.len(),
		"expected {}-part date array for {key} in {body}",
		expected.len()
	);
	for (idx, expected_part) in expected.iter().enumerate() {
		assert_eq!(
			actual[idx].as_i64(),
			Some(*expected_part),
			"date element {} mismatch for {key} in {body}",
			idx
		);
	}
}
