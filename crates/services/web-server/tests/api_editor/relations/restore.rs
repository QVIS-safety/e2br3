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
	let initial_replay = json!({"authorities":["ich"],"rows":{owner:{"id":row_id,"sequenceNumber":1,input_field:initial}}});
	let initial_audits =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?;
	for attempt in 0..2 {
		let (status, replayed) =
			patch_json(&app, &cookie, &row_uri, initial_replay.clone()).await?;
		assert_eq!(status, StatusCode::OK, "replay {attempt}: {replayed}");
		let (fresh_status, fresh) = get_json(&app, &cookie, &row_uri).await?;
		assert_eq!(fresh_status, StatusCode::OK, "{fresh}");
		assert_eq!(fresh["data"][owner]["id"], row_id);
		assert_eq!(fresh["data"][owner][column], initial);
		let (page_status, page) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{page_id}"),
		)
		.await?;
		assert_eq!(page_status, StatusCode::OK, "{page}");
		let rows = page["rows"]["rows"]
			.as_array()
			.ok_or("missing repeatable rows")?;
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0]["id"], row_id);
		assert_eq!(
			crate::case_editor_contract_web::audit::case_audit_ids(
				&mm, &ctx, &case_id
			)
			.await?,
			initial_audits
		);
	}
	let collection_owner = match page_id {
		"DH" => "pastDrugHistory",
		"LR" => "literatureReferences",
		_ => return Err("unexpected restore page".into()),
	};
	for rows in [json!({}), json!({collection_owner:[]})] {
		let (status, _) = patch_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{page_id}"),
			json!({"authorities":["ich"],"rows":rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK);
		let (fresh_status, fresh) = get_json(&app, &cookie, &row_uri).await?;
		assert_eq!(fresh_status, StatusCode::OK, "{fresh}");
		assert_eq!(fresh["data"][owner]["id"], row_id);
		assert_eq!(fresh["data"][owner][column], initial);
		let (page_status, page) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{page_id}"),
		)
		.await?;
		assert_eq!(page_status, StatusCode::OK, "{page}");
		let listed = page["rows"]["rows"]
			.as_array()
			.ok_or("missing repeatable rows")?;
		assert_eq!(listed.len(), 1, "{page}");
		assert_eq!(listed[0]["id"], row_id, "{page}");
		assert_eq!(
			crate::case_editor_contract_web::audit::case_audit_ids(
				&mm, &ctx, &case_id
			)
			.await?,
			initial_audits
		);
	}
	let foreign_case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-{page_id}-FOREIGN"),
		&["ich"],
	)
	.await?;
	let wrong_rows_uri =
		format!("/api/cases/{foreign_case_id}/editor/pages/{page_id}/rows");
	let wrong_row_uri = format!("{wrong_rows_uri}/{row_id}");
	let original_before_scope = get_json(&app, &cookie, &row_uri).await?;
	assert_eq!(original_before_scope.0, StatusCode::OK);
	let original_list_before_scope = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/{page_id}"),
	)
	.await?;
	assert_eq!(original_list_before_scope.0, StatusCode::OK);
	let foreign_list_before_scope = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{foreign_case_id}/editor/pages/{page_id}"),
	)
	.await?;
	assert_eq!(foreign_list_before_scope.0, StatusCode::OK);
	let original_audits_before_scope =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?;
	let foreign_audits_before_scope =
		crate::case_editor_contract_web::audit::case_audit_ids(
			&mm,
			&ctx,
			&foreign_case_id,
		)
		.await?;
	let (status, body) = get_json(&app, &cookie, &wrong_row_uri).await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&wrong_row_uri,
		json!({"authorities":["ich"],"rows":{owner:{
			"id":row_id,"sequenceNumber":1,input_field:conflicting
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&wrong_row_uri)
				.header("cookie", &cookie)
				.header(
					"x-e2br3-reason-for-change",
					crate::case_editor_contract_web::support::ROW_CHANGE_REASON,
				)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
	assert_eq!(
		get_json(&app, &cookie, &row_uri).await?,
		original_before_scope
	);
	assert_eq!(
		get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/{page_id}"),
		)
		.await?,
		original_list_before_scope
	);
	assert_eq!(
		get_json(
			&app,
			&cookie,
			&format!("/api/cases/{foreign_case_id}/editor/pages/{page_id}"),
		)
		.await?,
		foreign_list_before_scope
	);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?,
		original_audits_before_scope
	);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(
			&mm,
			&ctx,
			&foreign_case_id,
		)
		.await?,
		foreign_audits_before_scope
	);
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

	super::assert_row_lifecycle_replay(
		&app,
		&cookie,
		&mm,
		&seed,
		&lifecycle,
		&format!("/api/cases/{case_id}/editor/pages/{page_id}"),
		&json!({"authorities": ["ich"], "rows": {owner: {
			"id": row_id, "deleted": false, input_field: restored
		}}}),
	)
	.await?;

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
