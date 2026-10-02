use super::support::field_contract_test;
const PAGE_ID: &str = "LB";
include!("generated/lb_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn comments_can_be_cleared_and_reloaded() -> crate::common::Result<()> {
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
		create_case_for_editor(&app, &cookie, "EDITOR-LB-CLEAR", &["ich"]).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/test-results"),
		json!({"data": {"case_id": case_id, "sequence_number": 1, "test_name": "Fixture"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let test_id = body["data"]["id"]
		.as_str()
		.ok_or("missing test result id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/LB/rows/{test_id}");

	for comments in [json!("Lab comment"), Value::Null] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {"testResult": {"comments": comments}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["testResult"]["comments"], Value::Null);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn clearing_numeric_result_clears_qualifier_and_keeps_free_text(
) -> crate::common::Result<()> {
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
		create_case_for_editor(&app, &cookie, "EDITOR-LB-NUMERIC-CLEAR", &["ich"])
			.await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LB/rows"),
		json!({"authorities": ["ich"], "rows": {"testResult": {
			"sequenceNumber": 1,
			"testName": "Fixture",
			"testResult": "5.5",
			"testResultQualifier": "LE",
			"testResultUnstructured": "Free result"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let test_id = body["rowId"].as_str().ok_or("missing test result id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/LB/rows/{test_id}");
	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"], "5.5",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"], "LE",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["result_unstructured"], "Free result",
		"{body}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResult": "5.6"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"], "5.6",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"], "LE",
		"{body}"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResultQualifier": "GE"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"], "5.6",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"], "GE",
		"{body}"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResultQualifier": null}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"], "5.6",
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"],
		Value::Null,
		"{body}"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResultQualifier": "LE"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResult": null}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"],
		Value::Null,
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"],
		Value::Null,
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["result_unstructured"], "Free result",
		"{body}"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {"testResultQualifier": "LE"}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_result_value"],
		Value::Null,
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["test_result_qualifier"],
		Value::Null,
		"{body}"
	);
	assert_eq!(
		body["data"]["testResult"]["result_unstructured"], "Free result",
		"{body}"
	);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn list_projection_includes_displayed_result_columns(
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
		create_case_for_editor(&app, &cookie, "EDITOR-LB-LIST", &["ich"]).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/test-results"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"test_name": "Fixture",
			"test_result_code": "1",
			"test_result_value": "12.3",
			"result_unstructured": "Free result",
			"comments": "Lab comment"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/LB"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let row = &body["rows"]["rows"][0];
	assert_eq!(row["testResultCode"], "1");
	assert_eq!(row["testResult"], "12.3");
	assert_eq!(row["testResultUnstructured"], "Free result");
	assert_eq!(row["comments"], "Lab comment");
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn deleted_test_result_can_be_read_and_restored_with_case_and_auth_guards(
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
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &admin_cookie, "EDITOR-LB-RESTORE", &["ich"])
			.await?;
	let other_case_id = create_case_for_editor(
		&app,
		&admin_cookie,
		"EDITOR-LB-RESTORE-OTHER",
		&["ich"],
	)
	.await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/LB/rows");
	let (status, body) = post_json(
		&app,
		&admin_cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {
			"sequenceNumber": 1, "testName": "Deleted lab result"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing LB row id")?;
	let row_uri = format!("{rows_uri}/{row_id}");
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&row_uri)
				.header("cookie", &admin_cookie)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
	let (status, body) = get_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{case_id}/editor/pages/LB"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(
		body["rows"]["rows"]
			.as_array()
			.is_some_and(|rows| rows.iter().all(|row| row["id"] != row_id)),
		"{body}"
	);

	let (status, body) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["testResult"]["deleted"], true, "{body}");

	let restore = json!({"authorities": ["ich"], "rows": {"testResult": {
		"id": row_id, "deleted": false, "testName": "Restored lab result"
	}}});
	let wrong_case_uri =
		format!("/api/cases/{other_case_id}/editor/pages/LB/rows/{row_id}");
	let (status, _) = get_json(&app, &admin_cookie, &wrong_case_uri).await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) =
		patch_json(&app, &admin_cookie, &wrong_case_uri, restore.clone()).await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) =
		patch_json(&app, &viewer_cookie, &row_uri, restore.clone()).await?;
	assert_eq!(status, StatusCode::FORBIDDEN);

	let (status, _) = patch_json(
		&app,
		&admin_cookie,
		&row_uri,
		json!({"authorities": ["ich"], "rows": {"testResult": {
			"id": row_id,
			"deleted": false,
			"testName": "Must not persist",
			"testResultQualifier": "LE"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST);
	let (status, body) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["testResult"]["deleted"], true, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_name"], "Deleted lab result",
		"{body}"
	);

	let (status, body) = patch_json(&app, &admin_cookie, &row_uri, restore).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["testResult"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_name"], "Restored lab result",
		"{body}"
	);
	let (status, body) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["testResult"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["testResult"]["test_name"], "Restored lab result",
		"{body}"
	);
	let (status, body) = get_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{case_id}/editor/pages/LB"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(
		body["rows"]["rows"]
			.as_array()
			.is_some_and(|rows| rows.iter().any(|row| row["id"] == row_id
				&& row["testName"] == "Restored lab result")),
		"{body}"
	);
	Ok(())
}
