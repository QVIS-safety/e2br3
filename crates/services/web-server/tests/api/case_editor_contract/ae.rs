use super::support::field_contract_test;
const PAGE_ID: &str = "AE";
include!("generated/ae_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn split_null_flavor_http_contract() -> crate::common::Result<()> {
	super::support::verify_split_null_flavor_http_contract().await
}

#[serial_test::serial]
#[tokio::test]
async fn severity_can_be_cleared_and_reloaded() -> crate::common::Result<()> {
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
		create_case_for_editor(&app, &cookie, "EDITOR-AE-CLEAR", &["ich"]).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({"data": {"case_id": case_id, "sequence_number": 1}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let reaction_id = body["data"]["id"].as_str().ok_or("missing reaction id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/AE/rows/{reaction_id}");

	for severity in [json!("moderate"), Value::Null] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {"reaction": {"severity": severity}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["reaction"]["severity"], Value::Null);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn deleted_reaction_can_be_read_and_restored_with_case_and_auth_guards(
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
		create_case_for_editor(&app, &admin_cookie, "EDITOR-AE-RESTORE", &["ich"])
			.await?;
	let other_case_id = create_case_for_editor(
		&app,
		&admin_cookie,
		"EDITOR-AE-RESTORE-OTHER",
		&["ich"],
	)
	.await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/AE/rows");
	let (status, body) = post_json(
		&app,
		&admin_cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"reaction": {
			"sequenceNumber": 1,
			"primarySourceReaction": "Deleted reaction",
			"reactionMeddraCodeLLT": "10088978",
			"reactionMeddraVersionLLT": "28.1"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing AE row id")?;
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
		&format!("/api/cases/{case_id}/editor/pages/AE"),
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
	assert_eq!(body["data"]["reaction"]["deleted"], true, "{body}");

	let restore = json!({"authorities": ["ich"], "rows": {"reaction": {
		"id": row_id,
		"deleted": false,
		"primarySourceReaction": "Restored reaction"
	}}});
	let wrong_case_uri =
		format!("/api/cases/{other_case_id}/editor/pages/AE/rows/{row_id}");
	let (status, _) = get_json(&app, &admin_cookie, &wrong_case_uri).await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) =
		patch_json(&app, &admin_cookie, &wrong_case_uri, restore.clone()).await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) =
		patch_json(&app, &viewer_cookie, &row_uri, restore.clone()).await?;
	assert_eq!(status, StatusCode::FORBIDDEN);

	let invalid_value = "X".repeat(251);
	let (status, body) = patch_json(
		&app,
		&admin_cookie,
		&row_uri,
		json!({"authorities": ["ich"], "rows": {"reaction": {
			"id": row_id,
			"deleted": false,
			"primarySourceReaction": invalid_value
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION", "{body}");
	assert_eq!(
		body["error"]["data"]["detail"]["ruleCode"], "ICH.E.i.1.1a.LENGTH.MAX",
		"{body}"
	);
	assert_eq!(
		body["error"]["data"]["detail"]["path"], "reactions.0.primarySourceReaction",
		"{body}"
	);
	let (status, body) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["reaction"]["deleted"], true, "{body}");
	assert_eq!(
		body["data"]["reaction"]["primary_source_reaction"], "Deleted reaction",
		"{body}"
	);

	let (status, body) = patch_json(&app, &admin_cookie, &row_uri, restore).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["reaction"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["reaction"]["primary_source_reaction"], "Restored reaction",
		"{body}"
	);
	let (status, body) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["reaction"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["reaction"]["primary_source_reaction"], "Restored reaction",
		"{body}"
	);
	let (status, body) = get_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(
		body["rows"]["rows"]
			.as_array()
			.is_some_and(|rows| rows.iter().any(|row| row["id"] == row_id
				&& row["reactionPrimarySourceNative"] == "Restored reaction")),
		"{body}"
	);
	Ok(())
}
