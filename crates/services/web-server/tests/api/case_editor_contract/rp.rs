use super::support::field_contract_test;
const PAGE_ID: &str = "RP";
include!("generated/rp_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn reporter_validation_switches_from_collection_to_existing_fields(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::json;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "RP-COLLECTION-ERROR", &["ich"])
			.await?;
	let validation_uri = format!("/api/cases/{case_id}/validation?authority=ich");
	let (status, report) = get_json(&app, &cookie, &validation_uri).await?;
	assert_eq!(status, StatusCode::OK, "{report}");
	let paths: Vec<_> = report["data"]["issues"]
		.as_array()
		.unwrap()
		.iter()
		.filter_map(|issue| issue["path"].as_str())
		.filter(|path| path.starts_with("primarySources"))
		.collect();
	assert_eq!(paths, vec!["primarySources"]);
	assert_eq!(report["data"]["ok"], false);

	let (status, saved) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/RP"),
		json!({"authorities": ["ich"], "rows": {"primarySources": [{
			"reporterGivenName": "Reporter with missing qualification",
			"primarySourceForRegulatoryPurposes": "1"
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	assert_eq!(saved["rows"]["primarySources"].as_array().unwrap().len(), 1);
	let (status, report) = get_json(&app, &cookie, &validation_uri).await?;
	assert_eq!(status, StatusCode::OK, "{report}");
	let paths: Vec<_> = report["data"]["issues"]
		.as_array()
		.unwrap()
		.iter()
		.filter_map(|issue| issue["path"].as_str())
		.filter(|path| path.starts_with("primarySources"))
		.collect();
	assert!(!paths.contains(&"primarySources"), "{paths:?}");
	assert!(
		paths.contains(&"primarySources.0.qualification"),
		"{paths:?}"
	);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn reporter_title_can_be_cleared_without_changing_siblings(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::{json, Value};

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-RP-CLEAR", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/RP");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"primarySources": [{
			"reporterTitle": "Dr",
			"reporterGivenName": "Sibling"
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let source_id = body["rows"]["primarySources"][0]["id"]
		.as_str()
		.ok_or("missing primary source id")?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"primarySources": [{
			"id": source_id,
			"reporterTitle": null
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let source = &body["rows"]["primarySources"][0];
	assert_eq!(source["reporterTitle"], Value::Null);
	assert_eq!(source["reporterGivenName"], "Sibling");
	Ok(())
}
