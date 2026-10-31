use super::{
	assert_child_mutation_guards, assert_malformed_row_id_rejected_without_mutation,
	patch_and_reload,
};
use crate::case_editor_contract_web::audit::case_audit_ids;
use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use serde_json::{json, Value};

pub(crate) async fn ci_soft_children_delete_restore_and_resave_are_stable(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let app = web_server::app(mm.clone());
	let case_id =
		create_case_for_editor(&app, &cookie, "CI-CHILD-LIFECYCLE", &["ich"])
			.await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/CI");
	let definitions = [
		(
			"documentsHeldBySender",
			"documentDescription",
			json!({"documentDescription":"Document"}),
		),
		(
			"otherCaseIdentifiers",
			"caseIdentifier",
			json!({"source":"Source","caseIdentifier":"Other-ID"}),
		),
		(
			"linkedReports",
			"linkedReportNumber",
			json!({"linkedReportNumber":"Linked-ID"}),
		),
	];
	let mut creates = json!({});
	for (owner, _, row) in &definitions {
		let mut first = row.clone();
		first["sequenceNumber"] = json!(1);
		let mut sibling = row.clone();
		sibling["sequenceNumber"] = json!(2);
		creates[owner] = json!([first, sibling]);
	}
	let (status, saved) = patch_and_reload(
		&app,
		&cookie,
		&mm,
		&ctx,
		&uri,
		json!({"authorities":["ich"],"rows":creates.clone()}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let mut replay = creates;
	for (owner, _, _) in &definitions {
		let persisted = saved["rows"][owner]
			.as_array()
			.ok_or("missing CI replay rows")?;
		for (request, row) in replay[owner]
			.as_array_mut()
			.ok_or("missing CI replay requests")?
			.iter_mut()
			.zip(persisted)
		{
			request["id"] = row["id"].clone();
		}
	}
	let replay_audit = case_audit_ids(&mm, &ctx, &case_id).await?;
	for attempt in 0..2 {
		let (status, after) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":replay.clone()}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "replay {attempt}: {after}");
		for &(owner, field, _) in &definitions {
			assert_eq!(after["rows"][owner].as_array().map(Vec::len), Some(2));
			let actual_rows = after["rows"][owner]
				.as_array()
				.ok_or("missing replayed CI rows")?;
			let expected_rows = saved["rows"][owner]
				.as_array()
				.ok_or("missing saved CI rows")?;
			for (actual, expected) in actual_rows.iter().zip(expected_rows) {
				assert_eq!(actual["id"], expected["id"]);
				assert_eq!(actual[field], expected[field]);
			}
		}
		assert_eq!(case_audit_ids(&mm, &ctx, &case_id).await?, replay_audit);
	}
	for &(owner, field, _) in &definitions {
		let original = saved["rows"][owner]
			.as_array()
			.ok_or("missing CI children")?;
		assert_eq!(original.len(), 2);
		let id = original[0]["id"].clone();
		let sibling = original[1].clone();
		assert_malformed_row_id_rejected_without_mutation(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			&case_id,
			"ich",
			json!({owner:[{"id":"not-a-uuid","deleted":true}]}),
		)
		.await?;
		let mut calls = vec![
			(json!({owner:[{"id":id,"deleted":true}]}), false),
			(json!({owner:[{"id":id,"deleted":true}]}), false),
			(json!({}), false),
			(json!({owner:[]}), false),
			(json!({owner:[{"id":id}]}), false),
		];
		let restore = json!({owner:[{"id":id,"deleted":false,field:"Restored"}]});
		calls.push((restore.clone(), true));
		calls.push((restore, true));
		calls.push((json!({owner:[{"id":id,field:"Restored"}]}), true));
		for (rows, visible) in calls {
			let (status, after) = patch_and_reload(
				&app,
				&cookie,
				&mm,
				&ctx,
				&uri,
				json!({"authorities":["ich"],"rows":rows}),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{owner}: {after}");
			let children = after["rows"][owner]
				.as_array()
				.ok_or("missing CI children after save")?;
			assert_eq!(children.len(), if visible { 2 } else { 1 }, "{after}");
			assert_eq!(
				children.iter().find(|row| row["id"] == sibling["id"]),
				Some(&sibling),
				"sibling changed"
			);
			let target = children.iter().find(|row| row["id"] == id);
			if visible {
				assert_eq!(target.ok_or("restored ID missing")?[field], "Restored");
			} else {
				assert!(target.is_none(), "deleted row resurfaced");
			}
		}
	}
	assert_child_mutation_guards(&app, &cookie, &mm, &seed, &uri, "ich", 3).await?;
	Ok(())
}

pub(crate) async fn ci_child_foreign_case_rejection_rolls_back_rows_and_audits(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let app = web_server::app(mm.clone());
	let first =
		create_case_for_editor(&app, &cookie, "CI-SCOPE-A", &["ich"]).await?;
	let second =
		create_case_for_editor(&app, &cookie, "CI-SCOPE-B", &["ich"]).await?;
	let mut snapshots = Vec::new();
	for case_id in [&first, &second] {
		let uri = format!("/api/cases/{case_id}/editor/pages/CI");
		let (status, saved) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":{
				"documentsHeldBySender":[{"documentDescription":"Document"}],
				"otherCaseIdentifiers":[{"source":"Source","caseIdentifier":"ID"}],
				"linkedReports":[{"linkedReportNumber":"LINK"}]
			}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{saved}");
		snapshots.push((uri, saved, case_audit_ids(&mm, &ctx, case_id).await?));
	}
	for (owner, field) in [
		("documentsHeldBySender", "documentDescription"),
		("otherCaseIdentifiers", "caseIdentifier"),
		("linkedReports", "linkedReportNumber"),
	] {
		let own_id = &snapshots[0].1["rows"][owner][0]["id"];
		let foreign_id = &snapshots[1].1["rows"][owner][0]["id"];
		for deleted in [None, Some(true), Some(false)] {
			let mut foreign = json!({"id":foreign_id,field:"ILLEGAL"});
			if let Some(deleted) = deleted {
				foreign["deleted"] = Value::Bool(deleted);
			}
			let (status, body) = patch_json(
				&app,
				&cookie,
				&snapshots[0].0,
				json!({"authorities":["ich"],"rows":{owner:[
					{"id":own_id,field:"MUST ROLL BACK"}, foreign
				]}}),
			)
			.await?;
			assert_eq!(status, StatusCode::BAD_REQUEST, "{owner}: {body}");
			for (index, case_id) in [&first, &second].into_iter().enumerate() {
				let (uri, before, audit_before) = &snapshots[index];
				let (status, after) = get_json(&app, &cookie, uri).await?;
				assert_eq!(status, StatusCode::OK, "{after}");
				assert_eq!(&after, before, "foreign row rejection changed a case");
				assert_eq!(&case_audit_ids(&mm, &ctx, case_id).await?, audit_before);
			}
		}
	}
	Ok(())
}
