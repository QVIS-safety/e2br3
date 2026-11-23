use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use axum::{body::Body, http::Request};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
async fn fixture() -> Result<(axum::Router, String, String, String)> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let case_id =
		create_case_for_editor(&app, &cookie, "LEGACY-DH-READ", &["ich"]).await?;
	let (status, patient) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({"data":{"case_id":case_id,"patient_initials":"DH"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{patient}");
	let patient_id = patient["data"]["id"].as_str().ok_or("missing patient id")?;
	let(status,body)=post_json(&app,&cookie,&format!("/api/cases/{case_id}/patient/past-drugs"),json!({"data":{"patient_id":patient_id,"sequence_number":1,"drug_name":"Legacy prior drug","indication_meddra_code":"10012345"}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let row_id = body["data"]["id"]
		.as_str()
		.ok_or("missing past drug id")?
		.to_string();
	Ok((app, cookie, case_id, row_id))
}
#[serial]
#[tokio::test]
async fn test_dh_legacy_list_projects_rows() -> Result<()> {
	let (app, cookie, case_id, row_id) = fixture().await?;
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/DH/list"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	let row = &body["rows"][0];
	assert_eq!(row["id"], row_id);
	assert_eq!(row["drugName"], "Legacy prior drug");
	assert_eq!(row["indication"], "10012345");
	Ok(())
}
#[serial]
#[tokio::test]
async fn test_dh_legacy_detail_projects_row() -> Result<()> {
	let (app, cookie, case_id, row_id) = fixture().await?;
	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/DH/{row_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["rowId"], row_id);
	let row = &body["data"]["patientInformation"]["pastDrugHistory"][0];
	assert_eq!(row["id"], row_id);
	assert_eq!(row["drug_name"], "Legacy prior drug");
	assert_eq!(row["deleted"], false);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_dh_deleted_rows_require_explicit_editor_inclusion() -> Result<()> {
	let (app, cookie, case_id, row_id) = fixture().await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DH");
	let row_uri = format!("{uri}/rows/{row_id}");
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("{uri}/rows"),
		json!({
			"authorities":["ich"], "rows":{"pastDrugHistory":{
				"sequenceNumber":2, "drugName":"Isolated sibling"
			}}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let (status, initial) =
		get_json(&app, &cookie, &format!("{uri}?include_rows=true")).await?;
	assert_eq!(status, StatusCode::OK, "{initial}");
	let initial_rows = initial["rows"]["rows"]
		.as_array()
		.ok_or("missing DH rows")?;
	assert_eq!(initial_rows.len(), 2);
	assert_eq!(initial_rows[0]["id"], row_id);
	let sibling_id = initial_rows[1]["id"].as_str().ok_or("missing sibling id")?;
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
	assert!(response.status().is_success());
	for deleted in [true, false] {
		if !deleted {
			let (status, body) = patch_json(
				&app,
				&cookie,
				&row_uri,
				json!({
					"authorities":["ich"], "rows":{"pastDrugHistory":{"deleted":false}}
				}),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{body}");
		}
		for include_deleted in [false, true] {
			let query = if include_deleted {
				"include_deleted=true"
			} else {
				"include_deleted=false"
			};
			let (status, projection) =
				get_json(&app, &cookie, &format!("{uri}?include_rows=true&{query}"))
					.await?;
			assert_eq!(status, StatusCode::OK, "{projection}");
			let mut expected = initial_rows.clone();
			expected[0]["deleted"] = json!(deleted);
			expected[1]["deleted"] = json!(false);
			if deleted && !include_deleted {
				expected.remove(0);
			}
			assert_eq!(projection["rows"]["rows"], json!(expected));
			assert_eq!(
				projection["rows"]["rowDetails"][sibling_id],
				initial["rows"]["rowDetails"][sibling_id]
			);
			if !deleted || include_deleted {
				let detail = &projection["rows"]["rowDetails"][&row_id]["data"]
					["pastDrugHistory"];
				let mut expected = initial["rows"]["rowDetails"][&row_id]["data"]
					["pastDrugHistory"]
					.clone();
				expected["deleted"] = json!(deleted);
				expected["updated_at"] = detail["updated_at"].clone();
				expected["updated_by"] = detail["updated_by"].clone();
				assert_eq!(detail, &expected);
			} else {
				assert!(projection["rows"]["rowDetails"].get(&row_id).is_none());
			}
		}
		// The legacy list has no include_deleted query contract and stays active-only.
		let (status, legacy) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/DH/list"),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{legacy}");
		let expected_active: Vec<_> = initial_rows
			.iter()
			.filter(|row| !deleted || row["id"] != row_id)
			.cloned()
			.collect();
		assert_eq!(legacy["rows"], json!(expected_active));
		let (status, active) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/patient/past-drugs"),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{active}");
		let rows = active["data"]
			.as_array()
			.ok_or("missing active past-drug list")?;
		assert_eq!(rows.len(), if deleted { 1 } else { 2 });
		assert!(rows.iter().all(|row| row["deleted"] == false));
	}
	Ok(())
}
