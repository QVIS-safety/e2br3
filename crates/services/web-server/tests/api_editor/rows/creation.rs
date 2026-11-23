use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

pub(crate) async fn empty_collections_do_not_create_parents(
	page_id: &str,
	check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let contract = crate::case_editor_contract_web::support::behavior_contract(
		page_id,
		"collections",
		check_id,
	)?;
	let cases = contract["parameters"]["cases"]
		.as_array()
		.ok_or("missing empty-collection cases")?;
	assert!(
		!cases.is_empty(),
		"empty-collection contract must exercise an input"
	);
	for case in cases {
		let section = page_id;
		let empty = case.get("rows").ok_or("missing empty-collection payload")?;
		let parent = case["absentParent"]
			.as_str()
			.ok_or("missing absent parent")?;

		let id =
			create_case_for_editor(&app, &cookie, "EMPTY-LIST", &["ich"]).await?;
		let uri = format!("/api/cases/{id}/editor/pages/{section}");
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":empty}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (_, saved) = get_json(&app, &cookie, &uri).await?;
		assert!(saved["rows"][parent].is_null(), "{saved}");
	}

	Ok(())
}

pub(crate) async fn parent_child_drafts_require_sequence(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	for owner in ["parentMedicalHistory", "parentPastDrugs"] {
		let id =
			create_case_for_editor(&app, &cookie, "EMPTY-ROW", &["ich"]).await?;
		let uri = format!("/api/cases/{id}/editor/pages/DM");
		let before = get_json(&app, &cookie, &uri).await?;
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{owner:[{}]}}),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
		assert_eq!(body["error"]["message"], "INVALID_REQUEST");
		assert_eq!(
			body["error"]["data"]["detail"],
			"sequenceNumber is required"
		);
		assert_eq!(get_json(&app, &cookie, &uri).await?, before);
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{owner:[{"sequenceNumber":1}]}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert_eq!(
			body["rows"][owner].as_array().map(Vec::len),
			Some(1),
			"{body}"
		);
		assert_eq!(body["rows"][owner][0]["sequence_number"], 1);
	}

	Ok(())
}

pub(crate) async fn incomplete_test_result_preserves_absent_name(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	for (section, owner, payload, column) in [(
		"LB",
		"testResult",
		json!({"sequenceNumber": 1, "testMeddraVersion": "28.1"}),
		"test_name",
	)] {
		let case_id =
			create_case_for_editor(&app, &cookie, "DRAFT-SAVE", &["ich"]).await?;
		let uri = format!("/api/cases/{case_id}/editor/pages/{section}/rows");
		let (status, body) = post_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {owner: payload}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{section}: {body}");
		let row_id = body["rowId"].as_str().ok_or("missing row id")?;
		let (status, saved) =
			get_json(&app, &cookie, &format!("{uri}/{row_id}")).await?;
		assert_eq!(status, StatusCode::OK, "{saved}");
		assert_eq!(
			saved["data"][owner][column],
			serde_json::Value::Null,
			"{saved}"
		);
		assert_eq!(
			saved["data"][owner]["test_meddra_version"], "28.1",
			"{saved}"
		);
		assert_eq!(saved["data"][owner]["sequence_number"], 1, "{saved}");
	}

	Ok(())
}

pub(crate) async fn blank_product_rejects_creation_without_a_row(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let case_id =
		create_case_for_editor(&app, &cookie, "DRAFT-SAVE", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DG");
	let (_, before) = get_json(&app, &cookie, &uri).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("{uri}/rows"),
		json!({"authorities": ["ich"], "rows": {"drug": {
			"drugCharacterization": "1", "medicinalProduct": ""
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(
		body["error"]["data"]["detail"],
		json!({
			"message": "empty strings are not allowed; use null to clear the field",
			"path": "drug.medicinalProduct",
			"ruleCode": "INPUT.EMPTY.REJECTED"
		})
	);
	let (_, after) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(after, before);

	let draft_case_id =
		create_case_for_editor(&app, &cookie, "INCOMPLETE-DG", &["ich"]).await?;
	let draft_uri = format!("/api/cases/{draft_case_id}/editor/pages/DG");
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("{draft_uri}/rows"),
		json!({"authorities": ["ich"], "rows": {"drug": {
			"sequenceNumber": 1,
			"manufacturerName": "Draft manufacturer"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing row id")?;
	let (status, saved) =
		get_json(&app, &cookie, &format!("{draft_uri}/rows/{row_id}")).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	assert!(saved["data"]["drug"]["drug_characterization"].is_null());
	assert!(saved["data"]["drug"]["medicinal_product"].is_null());
	let (status, report) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{draft_case_id}/validation?authority=ich"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{report}");
	let codes = report["data"]["issues"]
		.as_array()
		.ok_or("missing validation issues")?
		.iter()
		.filter_map(|issue| issue["code"].as_str())
		.collect::<Vec<_>>();
	assert!(codes.contains(&"ICH.G.k.1.REQUIRED"), "{report}");
	assert!(codes.contains(&"ICH.G.k.2.2.REQUIRED"), "{report}");

	Ok(())
}
