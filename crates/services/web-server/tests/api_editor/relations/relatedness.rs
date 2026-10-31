use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_row_with_reason as patch_json,
	post_json, ROW_CHANGE_REASON,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};

pub(crate) async fn relatedness_delete_preserves_sibling_and_parent_delete_removes_assessment(
	_page_id: &str,
	_check_id: &str,
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
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		"EDITOR-DG-ASSESSMENT-DELETE",
		&["ich"],
	)
	.await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Assessment delete drug"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}");

	let mut reaction_ids = Vec::new();
	for sequence_number in 1..=3 {
		let (status, body) = post_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/reactions"),
			json!({"data": {
				"case_id": case_id,
				"sequence_number": sequence_number,
				"primary_source_reaction": format!("Assessment reaction {sequence_number}")
			}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
		reaction_ids.push(
			body["data"]["id"]
				.as_str()
				.ok_or("missing reaction id")?
				.to_string(),
		);
	}

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [
				{"reactionId": reaction_ids[0], "sequenceNumber": 1, "sourceOfAssessment": "Reporter"},
				{"reactionId": reaction_ids[0], "sequenceNumber": 2, "sourceOfAssessment": "Sponsor"},
				{"reactionId": reaction_ids[1], "expectedness": "1"}
			]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let assessments = body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments")?;
	let sibling_rows = assessments
		.iter()
		.filter(|row| row["reactionId"] == json!(reaction_ids[0]))
		.collect::<Vec<_>>();
	assert_eq!(sibling_rows.len(), 2, "{body}");
	let relatedness_id = sibling_rows[0]["id"]
		.as_str()
		.ok_or("missing relatedness id")?
		.to_string();
	let assessment_id = sibling_rows[0]["drugReactionAssessmentId"]
		.as_str()
		.ok_or("missing assessment id")?
		.to_string();
	assert_eq!(
		sibling_rows[1]["drugReactionAssessmentId"],
		json!(assessment_id)
	);
	let sibling_id = sibling_rows[1]["id"]
		.as_str()
		.ok_or("missing sibling relatedness id")?
		.to_string();
	assert_ne!(relatedness_id, sibling_id);
	let parent_only = assessments
		.iter()
		.find(|row| row["reactionId"] == json!(reaction_ids[1]))
		.ok_or("missing parent-only assessment")?;
	let parent_only_id = parent_only["drugReactionAssessmentId"]
		.as_str()
		.ok_or("missing parent-only assessment id")?
		.to_string();
	assert!(parent_only.get("id").is_none());

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"drugReactionAssessmentId": parent_only_id,
				"reactionId": reaction_ids[1],
				"expectedness": null
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let cleared_parent = body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after erase-only patch")?
		.iter()
		.find(|row| row["reactionId"] == json!(reaction_ids[1]))
		.ok_or("missing parent-only assessment after erase-only patch")?;
	assert_eq!(cleared_parent["expectedness"], Value::Null, "{body}");

	let mut guarded = Vec::new();
	for deleted in [None, Some(true), Some(false)] {
		let mut row = json!({"id":relatedness_id,"drugReactionAssessmentId":assessment_id,
            "reactionId":reaction_ids[0],"sourceOfAssessment":"Guarded"});
		if let Some(deleted) = deleted {
			row["deleted"] = json!(deleted);
		}
		guarded.push((
			"PATCH",
			uri.clone(),
			json!({"authorities":["ich"],"rows":{"drug":{"drugReactionAssessments":[row]}}}),
		));
	}
	guarded.push(("PATCH", uri.clone(), json!({"authorities":["ich"],"rows":{"drug":{
        "drugReactionAssessments":[{"drugReactionAssessmentId":assessment_id,"reactionId":reaction_ids[0],"_deleteAssessment":true}]
    }}})));
	super::assert_editor_request_guards(&app, &cookie, &mm, &seed, &uri, &guarded)
		.await?;
	let (status, body) = super::patch_and_reload(
		&app,
		&cookie,
		&mm,
		&ctx,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"id": relatedness_id,
				"drugReactionAssessmentId": assessment_id,
				"reactionId": reaction_ids[0],
				"deleted": true
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let siblings_after_delete = body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after relatedness delete")?
		.iter()
		.filter(|row| row["reactionId"] == json!(reaction_ids[0]))
		.collect::<Vec<_>>();
	assert_eq!(siblings_after_delete.len(), 1, "{body}");
	assert_eq!(siblings_after_delete[0]["id"], json!(sibling_id), "{body}");
	for row in [
		json!({"id":relatedness_id,"drugReactionAssessmentId":assessment_id,"reactionId":reaction_ids[0],"deleted":true}),
		json!({"id":relatedness_id,"drugReactionAssessmentId":assessment_id,"reactionId":reaction_ids[0]}),
	] {
		let (status, after) = super::patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":{"drug":{"drugReactionAssessments":[row]}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{after}");
		let siblings: Vec<_> = after["data"]["drug"]["drugReactionAssessments"]
			.as_array()
			.ok_or("missing assessment replay")?
			.iter()
			.filter(|row| row["reactionId"] == json!(reaction_ids[0]))
			.collect();
		assert_eq!(
			siblings.len(),
			1,
			"ordinary resave restored a deleted relatedness"
		);
		assert_eq!(siblings[0]["id"], sibling_id);
	}
	let restore_relatedness = json!({"authorities": ["ich"], "rows": {"drug": {
		"drugReactionAssessments": [{
			"id": relatedness_id,
			"drugReactionAssessmentId": assessment_id,
			"reactionId": reaction_ids[0],
			"deleted": false,
			"sourceOfAssessment": "Restored reporter"
		}]
	}}});
	for attempt in 0..2 {
		let (status, body) = super::patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			restore_relatedness.clone(),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "restore attempt {attempt}: {body}");
		let (status, persisted) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{persisted}");
		assert_eq!(body["data"], persisted["data"], "restore readback mismatch");
		let body = persisted;
		let siblings = body["data"]["drug"]["drugReactionAssessments"]
			.as_array()
			.ok_or("missing assessments after relatedness restore")?
			.iter()
			.filter(|row| row["reactionId"] == json!(reaction_ids[0]))
			.collect::<Vec<_>>();
		assert_eq!(siblings.len(), 2, "{body}");
		assert!(
			siblings.iter().any(|row| {
				row["id"] == json!(relatedness_id)
					&& row["drugReactionAssessmentId"] == json!(assessment_id)
					&& row["sourceOfAssessment"] == "Restored reporter"
			}),
			"{body}"
		);
		assert!(
			siblings.iter().any(|row| row["id"] == json!(sibling_id)),
			"{body}"
		);
	}

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"reactionId": reaction_ids[2],
				"deleted": true
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	assert!(body["error"]["data"]["detail"]
		.as_str()
		.unwrap_or_default()
		.contains("id for deletion"));
	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after rejected deletion")?
		.iter()
		.all(|row| row["reactionId"] != json!(reaction_ids[2])));

	let delete_audit_before = super::case_audit_ids(&mm, &ctx, &case_id).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"drugReactionAssessmentId": parent_only_id,
				"reactionId": reaction_ids[1],
				"deleted": true,
				"_deleteAssessment": true
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	crate::case_editor_contract_web::audit::assert_hard_delete(
		&mm,
		&ctx,
		"drug_reaction_assessments",
		uuid::Uuid::parse_str(&parent_only_id)?,
		&delete_audit_before,
		ROW_CHANGE_REASON,
	)
	.await?;
	let (read_status, persisted) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(read_status, StatusCode::OK, "{persisted}");
	assert_eq!(body["data"], persisted["data"]);

	assert!(body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after parent delete")?
		.iter()
		.all(|row| row["reactionId"] != json!(reaction_ids[1])));
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"drugReactionAssessmentId": parent_only_id,
				"reactionId": reaction_ids[1],
				"deleted": false
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

	let delete_audit_before = super::case_audit_ids(&mm, &ctx, &case_id).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"drugReactionAssessmentId": assessment_id,
				"reactionId": reaction_ids[0],
				"_deleteAssessment": true
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	crate::case_editor_contract_web::audit::assert_hard_delete(
		&mm,
		&ctx,
		"drug_reaction_assessments",
		uuid::Uuid::parse_str(&assessment_id)?,
		&delete_audit_before,
		ROW_CHANGE_REASON,
	)
	.await?;
	let (read_status, persisted) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(read_status, StatusCode::OK, "{persisted}");
	assert_eq!(body["data"], persisted["data"]);

	assert!(body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after related parent delete")?
		.iter()
		.all(|row| row["reactionId"] != json!(reaction_ids[0])));
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugReactionAssessments": [{
				"id": relatedness_id,
				"drugReactionAssessmentId": assessment_id,
				"reactionId": reaction_ids[0],
				"deleted": false
			}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert!(body["data"]["drug"]["drugReactionAssessments"]
		.as_array()
		.ok_or("missing assessments after rejected child restore")?
		.iter()
		.all(|row| row["reactionId"] != json!(reaction_ids[0])));
	let before_replay = body;
	let audit_before_replay = super::case_audit_ids(&mm, &ctx, &case_id).await?;
	for row in [
		json!({"drugReactionAssessmentId":parent_only_id,"reactionId":reaction_ids[1],"deleted":true,"_deleteAssessment":true}),
		json!({"drugReactionAssessmentId":assessment_id,"reactionId":reaction_ids[0],"_deleteAssessment":true}),
	] {
		let (status, rejected) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{"drug":{"drugReactionAssessments":[row]}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{rejected}");
		let (status, after) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{after}");
		assert_eq!(after, before_replay);
		assert_eq!(
			super::case_audit_ids(&mm, &ctx, &case_id).await?,
			audit_before_replay,
			"repeated hard parent deletion must not add audit rows"
		);
	}
	Ok(())
}
