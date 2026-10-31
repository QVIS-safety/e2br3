pub(super) mod ae_restore;
pub(super) mod device_children;
pub(super) mod dg_restore;
pub(super) mod dg_rows;
pub(super) mod dm_rows;
pub(super) mod documents;
pub(super) mod lb_restore;
pub(super) mod nr_rows;
pub(super) mod parents;
pub(super) mod relatedness;
pub(super) mod restore;
pub(super) mod rp_rows;
pub(super) mod scope;
pub(super) mod si_rows;

// Shared by all five row verifiers so their existing setup also checks retries.
pub(super) async fn assert_row_lifecycle_replay(
	app: &axum::Router,
	cookie: &str,
	mm: &lib_core::model::ModelManager,
	ctx: &lib_core::ctx::Ctx,
	lifecycle: &serde_json::Value,
	page_uri: &str,
	restore_request: &serde_json::Value,
) -> crate::common::Result<()> {
	use super::{audit, support};
	use axum::body::Body;
	use axum::http::{Request, StatusCode};
	use serde_json::Value;
	use tower::ServiceExt;

	fn row_state(data: &Value, owner: &str) -> crate::common::Result<Value> {
		let mut row = data[owner]
			.as_object()
			.ok_or("missing persisted row")?
			.clone();
		// Updating an unchanged row may still touch its modification metadata.
		row.remove("updated_at");
		row.remove("updated_by");
		Ok(Value::Object(row))
	}

	let rows = restore_request["rows"].as_object().ok_or("missing rows")?;
	assert_eq!(rows.len(), 1);
	let (owner, row) = rows.iter().next().ok_or("missing row owner")?;
	let row_id = row["id"].as_str().ok_or("missing persisted row id")?;
	let row_uri = format!("{page_uri}/rows/{row_id}");
	let case_id = uuid::Uuid::parse_str(
		page_uri
			.strip_prefix("/api/cases/")
			.and_then(|rest| rest.split('/').next())
			.ok_or("invalid editor page URI")?,
	)?;
	let page = page_uri.rsplit('/').next().ok_or("missing page")?;
	let has_post_restore = matches!(page, "AE" | "LB" | "DG");
	let repeat_delete_status = if has_post_restore {
		StatusCode::NOT_FOUND
	} else {
		StatusCode::NO_CONTENT
	};
	let (status, original) = support::get_json(app, cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{original}");
	let original_row = row_state(&original["data"], owner)?;
	assert_eq!(original_row["id"], row_id);
	assert_eq!(original_row["deleted"], false);
	let (status, list) = support::get_json(app, cookie, page_uri).await?;
	assert_eq!(status, StatusCode::OK, "{list}");
	let original_ids: Vec<Value> = list["rows"]["rows"]
		.as_array()
		.ok_or("missing row list")?
		.iter()
		.map(|row| row["id"].clone())
		.collect();
	assert_eq!(
		original_ids
			.iter()
			.filter(|id| id.as_str() == Some(row_id))
			.count(),
		1
	);

	let mut ordinary_request = restore_request.clone();
	ordinary_request["rows"][owner]
		.as_object_mut()
		.ok_or("missing restore row")?
		.remove("deleted");
	let mut calls = vec![
		("PATCH", restore_request.clone(), StatusCode::OK, false),
		("PATCH", restore_request.clone(), StatusCode::OK, false),
		("DELETE", Value::Null, StatusCode::NO_CONTENT, true),
		("DELETE", Value::Null, repeat_delete_status, true),
		("PATCH", ordinary_request, StatusCode::OK, true),
	];
	if has_post_restore {
		calls.push(("POST", Value::Null, StatusCode::OK, false));
		calls.push(("POST", Value::Null, StatusCode::OK, false));
	} else {
		calls.push(("PATCH", restore_request.clone(), StatusCode::OK, false));
	}
	calls.push(("PATCH", restore_request.clone(), StatusCode::OK, false));
	for (method, request, expected_status, deleted) in calls {
		let (status, before) = support::get_json(app, cookie, &row_uri).await?;
		assert_eq!(status, StatusCode::OK, "{before}");
		let audit_before =
			audit::capture(mm, ctx, case_id, &lifecycle["audit"], &before["data"])
				.await?;
		let uri = if method == "POST" {
			format!("{row_uri}/restore")
		} else {
			row_uri.clone()
		};
		let body = if request.is_null() {
			Body::empty()
		} else {
			Body::from(request.to_string())
		};
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method(method)
					.uri(&uri)
					.header("cookie", cookie)
					.header("content-type", "application/json")
					.header("x-e2br3-reason-for-change", support::ROW_CHANGE_REASON)
					.body(body)?,
			)
			.await?;
		assert_eq!(response.status(), expected_status, "{method} {uri}");
		let (status, after) = support::get_json(app, cookie, &row_uri).await?;
		assert_eq!(status, StatusCode::OK, "{after}");
		let mut expected_row = original_row.clone();
		expected_row["deleted"] = Value::Bool(deleted);
		assert_eq!(
			row_state(&after["data"], owner)?,
			expected_row,
			"{method} {uri} changed identity, values or children"
		);
		audit::assert_mutation(
			mm,
			ctx,
			case_id,
			&lifecycle["audit"],
			audit_before,
			&after["data"],
			support::ROW_CHANGE_REASON,
		)
		.await?;
		let (status, list) = support::get_json(app, cookie, page_uri).await?;
		assert_eq!(status, StatusCode::OK, "{list}");
		let ids: Vec<Value> = list["rows"]["rows"]
			.as_array()
			.ok_or("missing row list")?
			.iter()
			.map(|row| row["id"].clone())
			.collect();
		let expected_ids: Vec<Value> = original_ids
			.iter()
			.filter(|id| !deleted || id.as_str() != Some(row_id))
			.cloned()
			.collect();
		assert_eq!(
			ids, expected_ids,
			"{method} {uri} changed the row count/identity"
		);
	}
	Ok(())
}

use super::audit::case_audit_ids;
use super::support::{get_json, patch_json};
use crate::common::Result;
use axum::http::StatusCode;
use lib_core::{ctx::Ctx, model::ModelManager};
use serde_json::{json, Value};

async fn patch_and_reload(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	payload: Value,
) -> Result<(StatusCode, Value)> {
	let (status, response) = patch_json(app, cookie, uri, payload).await?;
	if !status.is_success() {
		return Ok((status, response));
	}
	let (read_status, persisted) = get_json(app, cookie, uri).await?;
	assert_eq!(read_status, StatusCode::OK, "{persisted}");
	let key = if uri.contains("/rows/") {
		"data"
	} else {
		"rows"
	};
	assert_eq!(
		response.get(key).ok_or("PATCH projection missing")?,
		persisted.get(key).ok_or("GET projection missing")?,
		"PATCH response differs from persisted GET"
	);
	Ok((status, persisted))
}

// The rejected request must leave both the API projection and audit stream unchanged.
#[allow(clippy::too_many_arguments)]
async fn assert_malformed_row_id_rejected_without_mutation(
	app: &axum::Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	uri: &str,
	case_id: &str,
	authority: &str,
	rows: Value,
) -> Result<()> {
	let (status, before) = get_json(app, cookie, uri).await?;
	assert_eq!(status, StatusCode::OK, "{before}");
	let audit_before = case_audit_ids(mm, ctx, case_id).await?;
	let (status, body) = patch_and_reload(
		app,
		cookie,
		uri,
		json!({"authorities":[authority],"rows":rows}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (status, after) = get_json(app, cookie, uri).await?;
	assert_eq!(status, StatusCode::OK, "{after}");
	assert_eq!(after, before, "malformed row ID mutated the projection");
	assert_eq!(
		case_audit_ids(mm, ctx, case_id).await?,
		audit_before,
		"malformed row ID created audit entries"
	);
	Ok(())
}
