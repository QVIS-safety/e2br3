use crate::case_editor_contract_web::support::{
	create_case_for_editor, create_reaction_contract_fixture, get_json, patch_json,
	post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

#[tokio::test]
async fn repeatable_row_batch_rolls_back_data_and_audit_on_validation_failure(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let case_id =
		create_case_for_editor(&app, &cookie, "AE-BATCH", &["ich"]).await?;
	let row_id = create_reaction_contract_fixture(&app, &cookie, &case_id).await?;
	let row_uri = format!("/api/cases/{case_id}/editor/pages/AE/rows/{row_id}");
	let (_, before) = get_json(&app, &cookie, &row_uri).await?;
	let audit_before =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows"),
		json!({
			"authorities": ["ich"],
			"operations": [
				{
					"operation": "update",
					"rowId": row_id,
					"clientRowIndex": 0,
					"rows": {"reaction": {"primarySourceReaction": "must roll back"}}
				},
				{
					"operation": "create",
					"clientRowIndex": 1,
					"rows": {"reaction": {"reactionStartDate": "not-a-date"}}
				}
			]
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION", "{body}");

	let (_, after) = get_json(&app, &cookie, &row_uri).await?;
	assert_eq!(
		after, before,
		"the successful first operation must roll back"
	);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?,
		audit_before,
		"rolled-back batch must not leave audit rows"
	);
	Ok(())
}

#[tokio::test]
async fn repeatable_row_batch_rejects_the_whole_batch_for_permission_and_lifecycle(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let case_id =
		create_case_for_editor(&app, &admin_cookie, "AE-BATCH-GUARD", &["ich"])
			.await?;
	let row_id =
		create_reaction_contract_fixture(&app, &admin_cookie, &case_id).await?;
	let row_uri = format!("/api/cases/{case_id}/editor/pages/AE/rows/{row_id}");
	let batch_uri = format!("/api/cases/{case_id}/editor/pages/AE/rows");
	let request = json!({
		"authorities": ["ich"],
		"operations": [{
			"operation": "update",
			"rowId": row_id,
			"clientRowIndex": 0,
			"rows": {"reaction": {"primarySourceReaction": "must not persist"}}
		}]
	});
	let (_, before) = get_json(&app, &admin_cookie, &row_uri).await?;
	let audit_before =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?;

	let (status, body) =
		patch_json(&app, &viewer_cookie, &batch_uri, request.clone()).await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
	assert_eq!(body["error"]["message"], "PERMISSION_DENIED", "{body}");
	let (_, after) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(after, before);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?,
		audit_before
	);

	let (_, case) =
		get_json(&app, &admin_cookie, &format!("/api/cases/{case_id}")).await?;
	let (status, body) = post_json(
		&app,
		&admin_cookie,
		&format!("/api/cases/{case_id}/lock/toggle"),
		json!({"data": {"expected_status": case["data"]["status"]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let audit_locked =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?;

	let (status, body) =
		patch_json(&app, &admin_cookie, &batch_uri, request).await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	assert_eq!(body["error"]["message"], "INVALID_REQUEST", "{body}");
	assert_eq!(
		body["error"]["data"]["detail"], "locked cases are read-only",
		"{body}"
	);
	let (_, after) = get_json(&app, &admin_cookie, &row_uri).await?;
	assert_eq!(after, before);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &case_id)
			.await?,
		audit_locked
	);
	Ok(())
}

#[tokio::test]
async fn repeatable_row_batch_commits_all_operations_together() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "AE-BATCH-OK", &["ich"]).await?;
	let row_id = create_reaction_contract_fixture(&app, &cookie, &case_id).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 2,
			"primary_source_reaction": "batch delete"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let deleted_id = body["data"]["id"]
		.as_str()
		.ok_or("missing row to delete")?
		.to_string();

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows"),
		json!({
			"authorities": ["ich"],
			"operations": [
				{
					"operation": "update",
					"rowId": row_id,
					"clientRowIndex": 0,
					"rows": {"reaction": {"primarySourceReaction": "batch update"}}
				},
				{
					"operation": "delete",
					"rowId": deleted_id,
					"clientRowIndex": 1
				},
				{
					"operation": "create",
					"clientRowIndex": 2,
					"rows": {"reaction": {"sequenceNumber": 3, "primarySourceReaction": "batch create"}}
				}
			]
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["results"].as_array().map(Vec::len), Some(3), "{body}");
	assert_eq!(body["results"][0]["rowId"], row_id, "{body}");
	assert_eq!(body["results"][1]["rowId"], deleted_id, "{body}");
	let created_id = body["results"][2]["rowId"]
		.as_str()
		.ok_or("missing created row ID")?;

	let (_, updated) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows/{row_id}"),
	)
	.await?;
	assert_eq!(
		updated["data"]["reaction"]["primary_source_reaction"],
		"batch update"
	);
	let (_, created) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows/{created_id}"),
	)
	.await?;
	assert_eq!(
		created["data"]["reaction"]["primary_source_reaction"],
		"batch create"
	);
	let (status, _) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/AE/rows/{deleted_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	Ok(())
}
