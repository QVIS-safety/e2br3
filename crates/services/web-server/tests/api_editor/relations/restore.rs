pub(crate) async fn atomic_restore(
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

	let contract = crate::case_editor_contract_web::support::behavior_contract(
		page_id,
		"relations",
		check_id,
	)?;
	let parameters = &contract["parameters"];
	let owner = parameters["owner"].as_str().ok_or("missing row owner")?;
	let input_field = parameters["inputField"]
		.as_str()
		.ok_or("missing row input field")?;
	let column = parameters["column"].as_str().ok_or("missing row column")?;
	let initial = parameters["initial"]
		.as_str()
		.ok_or("missing initial value")?;
	let restored = parameters["restored"]
		.as_str()
		.ok_or("missing restored value")?;
	let conflicting = parameters["conflicting"]
		.as_str()
		.ok_or("missing conflicting value")?;
	let replacement = parameters["replacement"]
		.as_str()
		.ok_or("missing replacement value")?;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-DH-RESTORE", &["ich"]).await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/{page_id}/rows");

	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {owner: {
			"sequenceNumber": 1, input_field: initial
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing DH row id")?;
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
	let (status, audit_before) = get_json(&app, &cookie, &row_uri).await?;
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
				.header("cookie", &cookie)
				.header(
					"x-e2br3-reason-for-change",
					crate::case_editor_contract_web::support::ROW_CHANGE_REASON,
				)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
	let (status, audit_deleted) = get_json(&app, &cookie, &row_uri).await?;
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

	let (status, body) = patch_json(
		&app,
		&cookie,
		&row_uri,
		json!({"authorities": ["ich"], "rows": {owner: {
			"id": row_id, "deleted": false, input_field: restored
		}}}),
	)
	.await?;
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

	assert_eq!(body["data"][owner]["deleted"], false, "{body}");
	assert_eq!(body["data"][owner][column], restored, "{body}");

	let (status, body) = get_json(&app, &cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"][owner]["deleted"], false, "{body}");
	assert_eq!(body["data"][owner][column], restored, "{body}");
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/{page_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["rows"].as_array().ok_or("missing DH rows")?;
	assert!(
		rows.iter()
			.any(|row| { row["id"] == row_id && row[input_field] == restored }),
		"{body}"
	);

	// Reusing an active sequence makes the restore itself fail at the database
	// constraint. The authorized mutation transaction must keep the target
	// deleted and preserve the replacement row.
	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {owner: {
			"sequenceNumber": 2, input_field: conflicting
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let deleted_id = body["rowId"].as_str().ok_or("missing deleted DH row id")?;
	let deleted_uri = format!("{rows_uri}/{deleted_id}");
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&deleted_uri)
				.header("cookie", &cookie)
				.header(
					"x-e2br3-reason-for-change",
					crate::case_editor_contract_web::support::ROW_CHANGE_REASON,
				)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {owner: {
			"sequenceNumber": 2, input_field: replacement
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let replacement_id = body["rowId"]
		.as_str()
		.ok_or("missing replacement DH row id")?;

	let (status, _) = patch_json(
		&app,
		&cookie,
		&deleted_uri,
		json!({"authorities": ["ich"], "rows": {owner: {
			"id": deleted_id, "deleted": false, input_field: "Must roll back"
		}}}),
	)
	.await?;
	assert!(
		!status.is_success(),
		"conflicting restore unexpectedly succeeded"
	);

	let (status, body) = get_json(&app, &cookie, &deleted_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"][owner]["deleted"], true, "{body}");
	assert_eq!(body["data"][owner][column], conflicting, "{body}");
	let (status, body) =
		get_json(&app, &cookie, &format!("{rows_uri}/{replacement_id}")).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"][owner]["deleted"], false, "{body}");
	assert_eq!(body["data"][owner][column], replacement, "{body}");
	Ok(())
}
