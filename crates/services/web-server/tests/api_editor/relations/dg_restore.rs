use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_row_with_reason as patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

pub(crate) async fn deleted_drug_can_be_read_and_restored_with_children_and_guards(
	page_id: &str,
	check_id: &str,
) -> Result<()> {
	use axum::body::Body;
	use axum::http::Request;
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
		create_case_for_editor(&app, &admin_cookie, "EDITOR-DG-RESTORE", &["ich"])
			.await?;
	let other_case_id = create_case_for_editor(
		&app,
		&admin_cookie,
		"EDITOR-DG-RESTORE-OTHER",
		&["ich"],
	)
	.await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/DG/rows");
	let (status, body) = post_json(
		&app,
		&admin_cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"sequenceNumber": 1,
			"drugCharacterization": "1",
			"medicinalProduct": "Deleted drug"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing DG row id")?;
	let row_uri = format!("{rows_uri}/{row_id}");
	let (status, body) = post_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{case_id}/drugs/{row_id}/device-characteristics"),
		json!({"data": {
			"drug_id": row_id,
			"sequence_number": 1,
			"code": "KR_DVC_MFR",
			"value_type": "ST",
			"value_value": "Preserved child"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let (status, body) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let expected_children = body["data"]["drug"]["deviceCharacteristics"].clone();
	assert_eq!(
		expected_children.as_array().map(Vec::len),
		Some(1),
		"{body}"
	);

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
	let initial_replay = json!({"authorities":["ich"],"rows":{"drug":{"id":row_id,"sequenceNumber":1,"drugCharacterization":"1","medicinalProduct":"Deleted drug"}}});
	let initial_audits =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?;
	for attempt in 0..2 {
		let (status, replayed) =
			patch_json(&app, &admin_cookie, &row_uri, initial_replay.clone())
				.await?;
		assert_eq!(status, StatusCode::OK, "replay {attempt}: {replayed}");
		let (fresh_status, fresh) = get_json(&app, &admin_cookie, &row_uri).await?;
		assert_eq!(fresh_status, StatusCode::OK, "{fresh}");
		assert_eq!(fresh["data"]["drug"]["id"], row_id);
		assert_eq!(fresh["data"]["drug"]["medicinal_product"], "Deleted drug");
		assert_eq!(
			fresh["data"]["drug"]["deviceCharacteristics"],
			expected_children
		);
		let (page_status, page) = get_json(
			&app,
			&admin_cookie,
			&format!("/api/cases/{case_id}/editor/pages/DG"),
		)
		.await?;
		assert_eq!(page_status, StatusCode::OK, "{page}");
		let rows = page["rows"]["rows"].as_array().ok_or("missing DG rows")?;
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
	for rows in [json!({}), json!({"drugs":[]})] {
		let (status, _) = patch_json(
			&app,
			&admin_cookie,
			&format!("/api/cases/{case_id}/editor/pages/DG"),
			json!({"authorities":["ich"],"rows":rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK);
		let (fresh_status, fresh) = get_json(&app, &admin_cookie, &row_uri).await?;
		assert_eq!(fresh_status, StatusCode::OK, "{fresh}");
		assert_eq!(fresh["data"]["drug"]["id"], row_id);
		assert_eq!(fresh["data"]["drug"]["medicinal_product"], "Deleted drug");
		let (page_status, page) = get_json(
			&app,
			&admin_cookie,
			&format!("/api/cases/{case_id}/editor/pages/DG"),
		)
		.await?;
		assert_eq!(page_status, StatusCode::OK, "{page}");
		let listed = page["rows"]["rows"].as_array().ok_or("missing DG rows")?;
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
		&format!("/api/cases/{case_id}/editor/pages/DG"),
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
	assert_eq!(body["data"]["drug"]["deleted"], true, "{body}");
	assert_eq!(
		body["data"]["drug"]["deviceCharacteristics"], expected_children,
		"{body}"
	);

	let restore = json!({"authorities": ["ich"], "rows": {"drug": {
		"id": row_id,
		"deleted": false,
		"medicinalProduct": "Restored drug"
	}}});
	let scope_detail_before = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(scope_detail_before.0, StatusCode::OK);
	let scope_page_before = get_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{case_id}/editor/pages/DG"),
	)
	.await?;
	assert_eq!(scope_page_before.0, StatusCode::OK);
	let scope_other_page_before = get_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{other_case_id}/editor/pages/DG"),
	)
	.await?;
	assert_eq!(scope_other_page_before.0, StatusCode::OK);
	let scope_audits_before =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?;
	let scope_other_audits_before =
		crate::case_editor_contract_web::audit::case_audit_ids(
			&mm,
			&ctx,
			&other_case_id,
		)
		.await?;
	let wrong_case_uri =
		format!("/api/cases/{other_case_id}/editor/pages/DG/rows/{row_id}");
	let (status, _) = get_json(&app, &admin_cookie, &wrong_case_uri).await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) =
		patch_json(&app, &admin_cookie, &wrong_case_uri, restore.clone()).await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) =
		patch_json(&app, &viewer_cookie, &row_uri, restore.clone()).await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	let (status, _) = post_json(
		&app,
		&admin_cookie,
		&format!("{wrong_case_uri}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) = post_json(
		&app,
		&viewer_cookie,
		&format!("{row_uri}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	assert_eq!(
		get_json(&app, &admin_cookie, &row_uri).await?,
		scope_detail_before
	);
	assert_eq!(
		get_json(
			&app,
			&admin_cookie,
			&format!("/api/cases/{case_id}/editor/pages/DG"),
		)
		.await?,
		scope_page_before
	);
	assert_eq!(
		get_json(
			&app,
			&admin_cookie,
			&format!("/api/cases/{other_case_id}/editor/pages/DG"),
		)
		.await?,
		scope_other_page_before
	);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?,
		scope_audits_before
	);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(
			&mm,
			&ctx,
			&other_case_id,
		)
		.await?,
		scope_other_audits_before
	);

	let invalid_value = "X".repeat(251);
	let (status, body) = patch_json(
		&app,
		&admin_cookie,
		&row_uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"id": row_id,
			"deleted": false,
			"medicinalProduct": invalid_value
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION", "{body}");
	assert_eq!(
		body["error"]["data"]["detail"]["ruleCode"], "ICH.G.k.2.2.LENGTH.MAX",
		"{body}"
	);
	assert_eq!(
		body["error"]["data"]["detail"]["path"], "drugs.0.medicinalProduct",
		"{body}"
	);
	let (status, body) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["drug"]["deleted"], true, "{body}");
	assert_eq!(
		body["data"]["drug"]["medicinal_product"], "Deleted drug",
		"{body}"
	);
	assert_eq!(
		body["data"]["drug"]["deviceCharacteristics"], expected_children,
		"{body}"
	);

	let (status, body) =
		patch_json(&app, &admin_cookie, &row_uri, restore.clone()).await?;
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

	assert_eq!(body["data"]["drug"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["drug"]["medicinal_product"], "Restored drug",
		"{body}"
	);
	assert_eq!(
		body["data"]["drug"]["deviceCharacteristics"], expected_children,
		"{body}"
	);
	let (status, body) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["drug"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["drug"]["deviceCharacteristics"], expected_children,
		"{body}"
	);
	let (status, body) = get_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{case_id}/editor/pages/DG"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(
		body["rows"]["rows"]
			.as_array()
			.is_some_and(|rows| rows.iter().any(|row| row["id"] == row_id
				&& row["medicinalProduct"] == "Restored drug")),
		"{body}"
	);
	super::assert_row_lifecycle_replay(
		&app,
		&admin_cookie,
		&mm,
		&seed,
		&lifecycle,
		&format!("/api/cases/{case_id}/editor/pages/DG"),
		&restore,
	)
	.await?;
	Ok(())
}
