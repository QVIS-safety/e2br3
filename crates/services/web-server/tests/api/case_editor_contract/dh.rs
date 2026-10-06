use super::support::field_contract_test;
const PAGE_ID: &str = "DH";
include!("generated/dh_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn page_projection_orders_past_drugs_by_sequence_then_id(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, post_json};
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
		create_case_for_editor(&app, &cookie, "EDITOR-DH-ORDER", &["ich"]).await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/DH/rows");
	for (sequence, drug_name) in [(2, "Second prior drug"), (1, "First prior drug")]
	{
		let (status, body) = post_json(
			&app,
			&cookie,
			&rows_uri,
			json!({"authorities": ["ich"], "rows": {"pastDrugHistory": {
				"sequenceNumber": sequence, "drugName": drug_name
			}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
	}

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/DH"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["rows"].as_array().ok_or("missing DH rows")?;
	let drug_names = rows
		.iter()
		.map(|row| row["drugName"].as_str())
		.collect::<Vec<_>>();
	assert_eq!(
		drug_names,
		vec![Some("First prior drug"), Some("Second prior drug")]
	);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn drug_name_can_be_cleared_and_reloaded() -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json, post_json};
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
		create_case_for_editor(&app, &cookie, "EDITOR-DH-CLEAR", &["ich"]).await?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({"data": {"case_id": case_id, "patient_initials": "FIXTURE"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let patient_id = body["data"]["id"].as_str().ok_or("missing patient id")?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient/past-drugs"),
		json!({"data": {
			"patient_id": patient_id,
			"sequence_number": 1,
			"drug_name": "Clearable prior drug"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["data"]["id"].as_str().ok_or("missing past drug id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DH/rows/{row_id}");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {
			"pastDrugHistory": {"drugName": null}
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["pastDrugHistory"]["drug_name"], Value::Null);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn deleted_row_can_be_restored_and_edited_atomically(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json, post_json};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::body::Body;
	use axum::http::{Request, StatusCode};
	use lib_auth::token::generate_web_token;
	use serde_json::json;
	use tower::ServiceExt;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-DH-RESTORE", &["ich"]).await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/DH/rows");

	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"pastDrugHistory": {
			"sequenceNumber": 1, "drugName": "Deleted prior drug"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["rowId"].as_str().ok_or("missing DH row id")?;
	let row_uri = format!("{rows_uri}/{row_id}");
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&row_uri)
				.header("cookie", &cookie)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&row_uri,
		json!({"authorities": ["ich"], "rows": {"pastDrugHistory": {
			"id": row_id, "deleted": false, "drugName": "Restored and edited"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["pastDrugHistory"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["pastDrugHistory"]["drug_name"], "Restored and edited",
		"{body}"
	);

	let (status, body) = get_json(&app, &cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["pastDrugHistory"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["pastDrugHistory"]["drug_name"], "Restored and edited",
		"{body}"
	);
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/DH"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let rows = body["rows"]["rows"].as_array().ok_or("missing DH rows")?;
	assert!(
		rows.iter().any(|row| {
			row["id"] == row_id && row["drugName"] == "Restored and edited"
		}),
		"{body}"
	);

	// Reusing an active sequence makes the restore itself fail at the database
	// constraint. The authorized mutation transaction must keep the target
	// deleted and preserve the replacement row.
	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"pastDrugHistory": {
			"sequenceNumber": 2, "drugName": "Conflicting deleted drug"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let deleted_id = body["rowId"].as_str().ok_or("missing deleted DH row id")?;
	let deleted_uri = format!("{rows_uri}/{deleted_id}");
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&deleted_uri)
				.header("cookie", &cookie)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["ich"], "rows": {"pastDrugHistory": {
			"sequenceNumber": 2, "drugName": "Active replacement"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let replacement_id = body["rowId"]
		.as_str()
		.ok_or("missing replacement DH row id")?;

	let (status, _) = patch_json(
		&app,
		&cookie,
		&deleted_uri,
		json!({"authorities": ["ich"], "rows": {"pastDrugHistory": {
			"id": deleted_id, "deleted": false, "drugName": "Must roll back"
		}}}),
	)
	.await?;
	assert!(
		!status.is_success(),
		"conflicting restore unexpectedly succeeded"
	);

	let (status, body) = get_json(&app, &cookie, &deleted_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["pastDrugHistory"]["deleted"], true, "{body}");
	assert_eq!(
		body["data"]["pastDrugHistory"]["drug_name"], "Conflicting deleted drug",
		"{body}"
	);
	let (status, body) =
		get_json(&app, &cookie, &format!("{rows_uri}/{replacement_id}")).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["data"]["pastDrugHistory"]["deleted"], false, "{body}");
	assert_eq!(
		body["data"]["pastDrugHistory"]["drug_name"], "Active replacement",
		"{body}"
	);
	Ok(())
}
