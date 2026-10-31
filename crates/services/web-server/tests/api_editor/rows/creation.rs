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
	let app = web_server::app(mm);
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

pub(crate) async fn empty_parent_child_objects_create_draft_rows(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	for owner in ["parentMedicalHistory", "parentPastDrugs"] {
		let id =
			create_case_for_editor(&app, &cookie, "EMPTY-ROW", &["ich"]).await?;
		let uri = format!("/api/cases/{id}/editor/pages/DM");
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{owner:[{}]}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert_eq!(
			body["rows"][owner].as_array().map(Vec::len),
			Some(1),
			"{body}"
		);
	}

	Ok(())
}

pub(crate) async fn incomplete_test_result_creates_empty_name(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	for (section, owner, payload, column) in [(
		"LB",
		"testResult",
		json!({"testMeddraVersion": "28.1"}),
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
		assert_eq!(saved["data"][owner][column], "", "{saved}");
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
	let app = web_server::app(mm);
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
			"message": "cannot be null",
			"path": "drugs.0.medicinalProduct",
			"ruleCode": "INPUT.NULL.REJECTED"
		})
	);
	let (_, after) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(after, before);

	Ok(())
}
