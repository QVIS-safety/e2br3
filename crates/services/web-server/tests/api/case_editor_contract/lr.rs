use super::support::field_contract_test;
const PAGE_ID: &str = "LR";
include!("generated/lr_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn page_projection_orders_references_by_sequence_then_id(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, post_json};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::json;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-LR-ORDER", &["ich"]).await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/LR/rows");
	for (sequence, reference) in [(2, "Second citation"), (1, "First citation")] {
		let (status, body) = post_json(
			&app,
			&cookie,
			&rows_uri,
			json!({"authorities": ["ich"], "rows": {"literatureReference": {
				"sequenceNumber": sequence, "referenceText": reference
			}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
	}

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LR"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["rows"].as_array().ok_or("missing LR rows")?;
	let references = rows
		.iter()
		.map(|row| row["referenceText"].as_str())
		.collect::<Vec<_>>();
	assert_eq!(references, vec![Some("First citation"), Some("Second citation")]);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn reference_text_can_be_cleared_and_reloaded() -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json, post_json};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::{json, Value};

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-LR-CLEAR", &["ich"]).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LR/rows"),
		json!({"authorities": ["ich"], "rows": {"literatureReference": {
			"sequenceNumber": 1,
			"referenceText": "Clearable reference"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing literature row id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/LR/rows/{row_id}");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {
			"literatureReference": {"referenceText": null}
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["literatureReference"]["reference_text"],
		Value::Null
	);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn deleted_reference_can_be_restored_and_edited_atomically(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json, post_json};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::body::Body;
	use axum::http::{Request, StatusCode};
	use lib_auth::token::generate_web_token;
	use serde_json::json;
	use tower::ServiceExt;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-LR-RESTORE", &["ich"]).await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/LR/rows");

	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"literatureReference": {
			"sequenceNumber": 1, "referenceText": "Deleted citation"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing LR row id")?;
	let row_uri = format!("{rows_uri}/{row_id}");
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&row_uri)
				.header("cookie", &cookie)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&row_uri,
		json!({"authorities": ["ich"], "rows": {"literatureReference": {
			"id": row_id, "deleted": false, "referenceText": "Restored and edited citation"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["literatureReference"]["deleted"], false,
		"{body}"
	);
	assert_eq!(
		body["data"]["literatureReference"]["reference_text"],
		"Restored and edited citation",
		"{body}"
	);

	let (status, body) = get_json(&app, &cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["literatureReference"]["deleted"], false,
		"{body}"
	);
	assert_eq!(
		body["data"]["literatureReference"]["reference_text"],
		"Restored and edited citation",
		"{body}"
	);
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LR"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["rows"].as_array().ok_or("missing LR rows")?;
	assert!(
		rows.iter().any(|row| {
			row["id"] == row_id
				&& row["referenceText"] == "Restored and edited citation"
		}),
		"{body}"
	);

	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"literatureReference": {
			"sequenceNumber": 2, "referenceText": "Conflicting deleted citation"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let deleted_id = body["rowId"].as_str().ok_or("missing deleted LR row id")?;
	let deleted_uri = format!("{rows_uri}/{deleted_id}");
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&deleted_uri)
				.header("cookie", &cookie)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"literatureReference": {
			"sequenceNumber": 2, "referenceText": "Active replacement citation"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let replacement_id = body["rowId"]
		.as_str()
		.ok_or("missing replacement LR row id")?;

	let (status, _) = patch_json(
		&app,
		&cookie,
		&deleted_uri,
		json!({"authorities": ["ich"], "rows": {"literatureReference": {
			"id": deleted_id, "deleted": false, "referenceText": "Must roll back"
		}}}),
	)
	.await?;
	assert!(
		!status.is_success(),
		"conflicting LR restore unexpectedly succeeded"
	);

	let (status, body) = get_json(&app, &cookie, &deleted_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["literatureReference"]["deleted"], true,
		"{body}"
	);
	assert_eq!(
		body["data"]["literatureReference"]["reference_text"],
		"Conflicting deleted citation",
		"{body}"
	);
	let (status, body) =
		get_json(&app, &cookie, &format!("{rows_uri}/{replacement_id}")).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["literatureReference"]["deleted"], false,
		"{body}"
	);
	assert_eq!(
		body["data"]["literatureReference"]["reference_text"],
		"Active replacement citation",
		"{body}"
	);
	Ok(())
}
