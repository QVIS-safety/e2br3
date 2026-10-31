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
	let (status, saved) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let doc = &saved["rows"]["sourceDocuments"][0];
	let doc_id = doc["id"].clone();
	let replay = json!({"sourceDocuments":[{"id":doc_id,"sourceDocumentName":"DOC","deleted":false}]});
	let replay_audits = super::case_audit_ids(&mm, &ctx, &id).await?;
	for attempt in 0..2 {
		let (status, after) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":replay.clone()}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "replay {attempt}: {after}");
		let (status, fresh) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{fresh}");
		let rows = fresh["rows"]["sourceDocuments"]
			.as_array()
			.ok_or("missing source documents")?;
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0]["id"], doc_id);
		assert_eq!(rows[0]["sourceDocumentName"], "DOC");
		assert_eq!(super::case_audit_ids(&mm, &ctx, &id).await?, replay_audits);
	}
	for rows in [json!({}), json!({"sourceDocuments":[]})] {
		let (status, _) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK);
		let (status, fresh) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{fresh}");
		assert_eq!(
			fresh["rows"]["sourceDocuments"].as_array().map(Vec::len),
			Some(1)
		);
		assert_eq!(fresh["rows"]["sourceDocuments"][0]["id"], doc_id);
		assert_eq!(
			fresh["rows"]["sourceDocuments"][0]["sourceDocumentName"],
			"DOC"
		);
		assert_eq!(super::case_audit_ids(&mm, &ctx, &id).await?, replay_audits);
	}
	let changed = json!({"sourceDocuments":[{
		"id":doc_id,"sourceDocumentName":"DOC-CHANGED","deleted":false
	}]});
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":changed.clone()}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let changed_audits = super::case_audit_ids(&mm, &ctx, &id).await?;
	for attempt in 0..2 {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":changed.clone()}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "changed replay {attempt}: {body}");
		let (status, fresh) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{fresh}");
		let rows = fresh["rows"]["sourceDocuments"]
			.as_array()
			.ok_or("missing source documents")?;
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0]["id"], doc_id);
		assert_eq!(rows[0]["sourceDocumentName"], "DOC-CHANGED");
		assert_eq!(super::case_audit_ids(&mm, &ctx, &id).await?, changed_audits);
	}
	let foreign_case_id = create_case_for_editor(
		&app,
		&cookie,
		"EDITOR-CONTRACT-FOREIGN-DOC",
		&["ich"],
	)
	.await?;
	let foreign_uri = format!("/api/cases/{foreign_case_id}/editor/pages/CI");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&foreign_uri,
		json!({"authorities":["ich"],"rows":{
			"sourceDocuments":[{"sourceDocumentName":"FOREIGN-DOC","deleted":false}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, foreign_before) = get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_before}");
	let foreign_doc_id = foreign_before["rows"]["sourceDocuments"][0]["id"].clone();
	let (status, original_before) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{original_before}");
	let original_audits = super::case_audit_ids(&mm, &ctx, &id).await?;
	let foreign_audits = super::case_audit_ids(&mm, &ctx, &foreign_case_id).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"sourceDocuments":[
			{"id":doc_id,"sourceDocumentName":"TARGET-MUST-NOT-CHANGE","deleted":false},
			{"id":foreign_doc_id,"sourceDocumentName":"FOREIGN-MUST-NOT-CHANGE","deleted":false}
		]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let (status, original_after) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{original_after}");
	assert_eq!(original_after, original_before);
	let (status, foreign_after) = get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_after}");
	assert_eq!(foreign_after, foreign_before);
	assert_eq!(
		super::case_audit_ids(&mm, &ctx, &id).await?,
		original_audits
	);
	assert_eq!(
		super::case_audit_ids(&mm, &ctx, &foreign_case_id).await?,
		foreign_audits
	);
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
