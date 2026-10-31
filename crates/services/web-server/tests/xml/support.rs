use crate::common::Result;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

pub(crate) fn workspace_root() -> std::path::PathBuf {
	std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

pub(crate) fn resolve_from_workspace(
	path: std::path::PathBuf,
) -> std::path::PathBuf {
	if path.is_absolute() {
		path
	} else {
		workspace_root().join(path)
	}
}

pub(crate) fn default_xsd_path() -> std::path::PathBuf {
	workspace_root()
		.join("docs/exporter/schema/multicacheschemas/MCCI_IN200100UV01.xsd")
}

pub(crate) fn resolved_xsd_path() -> std::path::PathBuf {
	match std::env::var("E2BR3_XSD_PATH") {
		Ok(value) => resolve_from_workspace(std::path::PathBuf::from(value)),
		Err(_) => default_xsd_path(),
	}
}

pub(crate) async fn request_json(
	app: &axum::Router,
	cookie: &str,
	method: &str,
	uri: String,
	body: Option<serde_json::Value>,
) -> Result<(StatusCode, Vec<u8>)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let req =
		builder.body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

pub(crate) async fn ensure_reaction_language(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		format!("/api/cases/{case_id}/reactions"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"list reactions status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let reactions =
		value.get("data").and_then(Value::as_array).ok_or_else(|| {
			format!("list reactions response missing data array: {value}")
		})?;
	for reaction in reactions {
		let reaction_id =
			reaction.get("id").and_then(Value::as_str).ok_or_else(|| {
				format!("reaction response item missing id: {reaction}")
			})?;
		let has_text = match reaction.get("primary_source_reaction") {
			None | Some(Value::Null) => false,
			Some(Value::String(value)) => !value.trim().is_empty(),
			Some(value) => {
				return Err(format!(
					"reaction primary_source_reaction is not a string or null: {value}"
				)
				.into())
			}
		};
		let has_language = match reaction.get("reaction_language") {
			None | Some(Value::Null) => false,
			Some(Value::String(value)) => !value.trim().is_empty(),
			Some(value) => {
				return Err(format!(
					"reaction reaction_language is not a string or null: {value}"
				)
				.into())
			}
		};
		if has_text && !has_language {
			let (status, body) = request_json(
				app,
				cookie,
				"PUT",
				format!("/api/cases/{case_id}/reactions/{reaction_id}"),
				Some(serde_json::json!({
					"data": { "reaction_language": "eng" }
				})),
			)
			.await?;
			if status != StatusCode::OK {
				return Err(format!(
					"update reaction language status {} body {}",
					status,
					String::from_utf8_lossy(&body)
				)
				.into());
			}
			let updated: Value = serde_json::from_slice(&body)?;
			if updated
				.get("data")
				.and_then(|data| data.get("reaction_language"))
				.and_then(Value::as_str)
				!= Some("eng")
			{
				return Err(format!(
					"update reaction language response did not contain data.reaction_language=eng: {updated}"
				)
				.into());
			}
		}
	}
	Ok(())
}

pub(crate) async fn ensure_batch_transmission_date(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<()> {
	let (status, body) = request_json(
		app,
		cookie,
		"GET",
		format!("/api/cases/{case_id}/message-header"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get message-header status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let header = value
		.get("data")
		.and_then(Value::as_object)
		.ok_or_else(|| {
			format!("message-header response missing data object: {value}")
		})?;
	let batch_transmission_date =
		header.get("batch_transmission_date").ok_or_else(|| {
			format!(
				"message-header response missing batch_transmission_date: {value}"
			)
		})?;
	let has_batch_transmission_date = if batch_transmission_date.is_null() {
		false
	} else {
		let parts = batch_transmission_date.as_array().ok_or_else(|| {
			format!(
				"message-header batch_transmission_date is not an array: {value}"
			)
		})?;
		if parts.is_empty() {
			false
		} else if parts.len() == 9 && parts.iter().all(Value::is_i64) {
			true
		} else {
			return Err(format!(
				"message-header batch_transmission_date is not a nine-integer timestamp: {value}"
			)
			.into());
		}
	};
	if has_batch_transmission_date {
		return Ok(());
	}
	let (status, body) = request_json(
		app,
		cookie,
		"PUT",
		format!("/api/cases/{case_id}/message-header"),
		Some(serde_json::json!({
			"data": {
				"batch_transmission_date": [2024, 32, 1, 1, 1, 0, 0, 0, 0]
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update batch_transmission_date status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let updated: Value = serde_json::from_slice(&body)?;
	if updated
		.get("data")
		.and_then(|data| data.get("batch_transmission_date"))
		!= Some(&serde_json::json!([2024, 32, 1, 1, 1, 0, 0, 0, 0]))
	{
		return Err(format!(
			"update batch_transmission_date response did not contain the requested timestamp: {updated}"
		)
		.into());
	}
	Ok(())
}
