use super::super::{
	audit,
	support::{create_case_for_editor, get_json},
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::{
	body::{to_bytes, Body},
	http::{Request, StatusCode},
};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

// N uses the message-header REST DTO, not an editor page payload.
pub(super) async fn verify_field(field: &Value) -> Result<()> {
	let code = field["code"].as_str().ok_or("missing N code")?;
	let column = field["projectionPath"]
		.as_str()
		.and_then(|path| path.strip_prefix("messageHeader."))
		.ok_or("missing N projection column")?;
	let mut valid = field
		.get("roundTripValue")
		.ok_or("missing roundtrip value")?
		.clone();
	if column == "batch_transmission_date" {
		let format = time::format_description::parse(
			"[year][month][day][hour][minute][second]",
		)?;
		let value = time::PrimitiveDateTime::parse(
			valid.as_str().ok_or("N date must be a string")?,
			&format,
		)?;
		valid = serde_json::to_value(value.assume_utc())?;
	}
	let invalid = field["constraint"]
		.get("invalidValue")
		.ok_or("missing invalid value")?;
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id =
		create_case_for_editor(&app, &cookie, "N-CONTRACT", &["ich"]).await?;
	let case_uuid = Uuid::parse_str(&case_id)?;
	if column == "message_number" {
		// Message numbers are globally unique, including pre-existing seed rows.
		valid = json!(format!(
			"{}-{case_id}",
			valid.as_str().ok_or("N message number must be a string")?
		));
	}
	let uri = format!("/api/cases/{case_id}/message-header");
	let create = json!({"case_id": case_id, "message_number": format!("MSG-{case_id}"),
        "batch_sender_identifier": "INITIAL", "batch_receiver_identifier": "INITIAL",
        "message_sender_identifier": "INITIAL", "message_receiver_identifier": "INITIAL",
        "message_date": "20240101000000"});

	// Batch number is derived on POST; its editable contract starts at PUT.
	if column != "batch_number" {
		let mut bad_create = create.clone();
		bad_create[column] = invalid.clone();
		let (status, bytes) =
			request(&app, &cookie, "POST", &uri, bad_create).await?;
		assert_rejection(field, column, status, &bytes)?;
		assert_eq!(
			get_json(&app, &cookie, &uri).await?.0,
			StatusCode::NOT_FOUND
		);
	}
	let (status, bytes) = request(&app, &cookie, "POST", &uri, create).await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{code}: {}",
		String::from_utf8_lossy(&bytes)
	);
	let (status, before) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK);
	let before = json!({"messageHeader": before["data"]});
	let audit_spec = json!({"table":"message_headers", "column":column,
        "record":{"source":"projection", "path":"messageHeader.id"}});
	let snapshot =
		audit::capture(&mm, &ctx, case_uuid, &audit_spec, &before).await?;
	let (status, bytes) =
		request(&app, &cookie, "PUT", &uri, json!({column: valid})).await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{code}: {}",
		String::from_utf8_lossy(&bytes)
	);
	let (status, saved) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(saved["data"][column], valid, "{code}");
	let saved_projection = json!({"messageHeader": saved["data"]});
	audit::assert_mutation(
		&mm,
		&ctx,
		case_uuid,
		&audit_spec,
		snapshot,
		&saved_projection,
		"N field contract",
	)
	.await?;

	let snapshot =
		audit::capture(&mm, &ctx, case_uuid, &audit_spec, &saved_projection).await?;
	let (status, bytes) =
		request(&app, &cookie, "PUT", &uri, json!({column: invalid})).await?;
	assert_rejection(field, column, status, &bytes)?;
	let (status, after) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(
		after, saved,
		"{code}: rejected update changed stored header"
	);
	audit::assert_rejected(&mm, &ctx, case_uuid, snapshot).await?;
	Ok(())
}

fn assert_rejection(
	field: &Value,
	column: &str,
	status: StatusCode,
	bytes: &[u8],
) -> Result<()> {
	assert_eq!(
		status,
		StatusCode::UNPROCESSABLE_ENTITY,
		"{}: {}",
		field["code"],
		String::from_utf8_lossy(bytes)
	);
	if column == "batch_transmission_date" {
		// This typed date fails deserialization before input-contract evaluation.
		assert!(std::str::from_utf8(bytes)?.contains("batch_transmission_date"));
	} else {
		let body: Value = serde_json::from_slice(bytes)?;
		assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION");
		assert_eq!(
			body["error"]["data"]["detail"]["ruleCode"],
			field["constraint"]["ruleCode"]
		);
		assert_eq!(
			body["error"]["data"]["detail"]["path"],
			field["frontendPath"]
		);
	}
	Ok(())
}

async fn request(
	app: &axum::Router,
	cookie: &str,
	method: &str,
	uri: &str,
	data: Value,
) -> Result<(StatusCode, Vec<u8>)> {
	let request = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.header("x-e2br3-reason-for-change", "N field contract")
		.body(Body::from(json!({"data":data}).to_string()))?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	Ok((
		status,
		to_bytes(response.into_body(), usize::MAX).await?.to_vec(),
	))
}
