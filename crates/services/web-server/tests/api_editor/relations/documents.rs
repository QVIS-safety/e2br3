use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_row_with_reason as patch_json,
	ROW_CHANGE_REASON,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

pub(crate) async fn source_document_delete_hides_the_target(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
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

	let id =
		create_case_for_editor(&app, &cookie, "EDITOR-CONTRACT", &["ich"]).await?;
	let base = format!("/api/cases/{id}/editor/pages");

	let uri = format!("{base}/CI");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"otherCaseIdentifiers":[{"source":null,"caseIdentifier":null}],
			"sourceDocuments":[{"sourceDocumentName":"DOC","deleted":false}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let doc = &saved["rows"]["sourceDocuments"][0];
	let mut guarded = Vec::new();
	for row in [
		json!({"id":doc["id"],"sourceDocumentName":"Changed"}),
		json!({"id":doc["id"],"deleted":true}),
		json!({"id":doc["id"],"deleted":false}),
	] {
		guarded.push((
			"PATCH",
			uri.clone(),
			json!({"authorities":["ich"],"rows":{"sourceDocuments":[row]}}),
		));
	}
	super::assert_editor_request_guards(&app, &cookie, &mm, &seed, &uri, &guarded)
		.await?;
	let audit_before = super::case_audit_ids(&mm, &ctx, &id).await?;
	let (status, body) = patch_json(&app, &cookie, &uri, json!({"authorities":["ich"],"rows":{"sourceDocuments":[{"id":doc["id"],"deleted":true}]}})).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	crate::case_editor_contract_web::audit::assert_hard_delete(
		&mm,
		&ctx,
		"source_documents",
		uuid::Uuid::parse_str(
			doc["id"].as_str().ok_or("missing source document ID")?,
		)?,
		&audit_before,
		ROW_CHANGE_REASON,
	)
	.await?;
	let audit_after = super::case_audit_ids(&mm, &ctx, &id).await?;
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		saved["rows"]["sourceDocuments"].as_array().map(Vec::len),
		Some(0),
		"{saved}"
	);
	for row in [
		json!({"id":doc["id"],"deleted":true}),
		json!({"id":doc["id"],"deleted":false,"sourceDocumentName":"must not restore"}),
		json!({"id":doc["id"],"sourceDocumentName":"must not recreate"}),
	] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{"sourceDocuments":[row]}}),
		)
		.await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
		let (status, after_replay) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{after_replay}");
		assert_eq!(after_replay, saved, "stale row replay changed CI");
		assert_eq!(
			super::case_audit_ids(&mm, &ctx, &id).await?,
			audit_after,
			"rejected replay wrote an audit"
		);
	}

	Ok(())
}
