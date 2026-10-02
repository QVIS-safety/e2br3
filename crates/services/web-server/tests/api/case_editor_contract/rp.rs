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

#[serial_test::serial]
#[tokio::test]
async fn reporters_remain_ordered_by_sequence_and_id_after_update(
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
		create_case_for_editor(&app, &cookie, "EDITOR-RP-ORDER", &["ich"])
			.await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/RP");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"primarySources": [
			{"sequenceNumber": 3, "reporterGivenName": "Sequence three"},
			{"sequenceNumber": 1, "reporterGivenName": "Sequence one"},
			{"sequenceNumber": 2, "reporterGivenName": "Sequence two"}
		]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["primarySources"]
		.as_array()
		.ok_or("missing primarySources")?;
	assert_eq!(rows.len(), 3, "{body}");
	let sequence_one_id = rows
		.iter()
		.find(|row| row["reporterGivenName"] == "Sequence one")
		.and_then(|row| row["id"].as_str())
		.ok_or("missing sequence-one reporter id")?
		.to_string();
	let sibling_rows: Vec<Value> = rows
		.iter()
		.filter(|row| row["id"] != sequence_one_id)
		.cloned()
		.collect();

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"primarySources": [{
			"id": sequence_one_id,
			"sequenceNumber": 1,
			"reporterPostcode": "04524"
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["primarySources"]
		.as_array()
		.ok_or("missing primarySources")?;
	assert_eq!(rows.len(), 3, "{body}");
	let order: Vec<_> = rows
		.iter()
		.map(|row| {
			(
				row["sequenceNumber"].as_i64(),
				row["id"].as_str(),
				row["reporterGivenName"].as_str(),
			)
		})
		.collect();
	assert_eq!(order[0].0, Some(1), "{body}");
	assert_eq!(order[0].1, Some(sequence_one_id.as_str()), "{body}");
	assert_eq!(order[0].2, Some("Sequence one"), "{body}");
	assert_eq!(rows[0]["reporterPostcode"], "04524", "{body}");
	assert_eq!(order[1].0, Some(2), "{body}");
	assert_eq!(order[1].2, Some("Sequence two"), "{body}");
	assert_eq!(order[2].0, Some(3), "{body}");
	assert_eq!(order[2].2, Some("Sequence three"), "{body}");
	for sibling in sibling_rows {
		let sibling_id = sibling["id"].as_str().ok_or("missing sibling id")?;
		let current = rows
			.iter()
			.find(|row| row["id"] == sibling_id)
			.ok_or("missing sibling after update")?;
		assert_eq!(current, &sibling, "sibling changed: {body}");
	}
	Ok(())
}
