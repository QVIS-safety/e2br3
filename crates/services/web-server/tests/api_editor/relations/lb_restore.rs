pub(crate) async fn deleted_test_result_can_be_read_and_restored_with_case_and_auth_guards(
	page_id: &str,
	check_id: &str,
) -> crate::common::Result<()> {
	use crate::case_editor_contract_web::support::{
		create_case_for_editor, get_json, patch_row_with_reason as patch_json,
		post_json,
	};
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
	let app = web_server::app(mm.clone());
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
	let lifecycle = crate::case_editor_contract_web::support::behavior_contract(
		page_id,
		"relations",
		check_id,
	)?;
	let audit_spec = &lifecycle["audit"];
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let case_uuid = uuid::Uuid::parse_str(&case_id)?;
	let (status, audit_before) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{audit_before}");
	let delete_before = crate::case_editor_contract_web::audit::capture(
		&mm,
		&ctx,
		case_uuid,
		audit_spec,
		&audit_before["data"],
	)
	.await?;
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&row_uri)
				.header("cookie", &admin_cookie)
				.header(
					"x-e2br3-reason-for-change",
					crate::case_editor_contract_web::support::ROW_CHANGE_REASON,
				)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
	let (status, audit_deleted) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{audit_deleted}");
	crate::case_editor_contract_web::audit::assert_mutation(
		&mm,
		&ctx,
		case_uuid,
		audit_spec,
		delete_before,
		&audit_deleted["data"],
		crate::case_editor_contract_web::support::ROW_CHANGE_REASON,
	)
	.await?;
	let restore_before = crate::case_editor_contract_web::audit::capture(
		&mm,
		&ctx,
		case_uuid,
		audit_spec,
		&audit_deleted["data"],
	)
	.await?;

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
	crate::case_editor_contract_web::audit::assert_mutation(
		&mm,
		&ctx,
		case_uuid,
		audit_spec,
		restore_before,
		&body["data"],
		crate::case_editor_contract_web::support::ROW_CHANGE_REASON,
	)
	.await?;

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
