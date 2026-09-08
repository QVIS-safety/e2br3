use super::support::{
	create_case_for_editor, field_contract_test, get_json, patch_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;
const PAGE_ID: &str = "SI";
include!("generated/si_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn study_sequence_gaps_keep_validation_bound_to_editor_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "SI-ROW-BINDING", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/SI");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({
			"authorities": ["ich"], "rows": {
				"studyInformation": {"studyName": "Sequence gaps"},
				"studyRegistrationNumbers": [
					{"registrationNumber": "LATER", "countryCode": "US", "sequenceNumber": 9},
					{"registrationNumber": "FIRST", "countryCode": "ZX", "sequenceNumber": 4}
				]
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["studyRegistrationNumbers"][0]["registration_number"],
		"FIRST"
	);
	assert_eq!(
		body["rows"]["studyRegistrationNumbers"][1]["registration_number"],
		"LATER"
	);
	let (status, report) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/validation?authority=ich"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{report}");
	let paths: Vec<_> = report["data"]["issues"]
		.as_array()
		.ok_or("missing issues")?
		.iter()
		.filter(|issue| issue["code"] == "ICH.C.5.1.r.2.VOCABULARY")
		.map(|issue| issue["path"].as_str().unwrap())
		.collect();
	assert_eq!(
		paths,
		vec!["studyInformation.0.registrations.0.registrationCountry"],
		"{report}"
	);
	Ok(())
}

#[serial_test::serial]
#[tokio::test]
async fn study_name_can_be_cleared_and_reloaded() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-SI-CLEAR", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/SI");

	for study_name in [json!("Clearable study"), serde_json::Value::Null] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities": ["ich"], "rows": {
				"studyInformation": {"studyName": study_name}
			}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["studyInformation"]["study_name"],
		serde_json::Value::Null
	);
	Ok(())
}

#[tokio::test]
async fn isolates_study_children_and_rejects_foreign_ids() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let first_case =
		create_case_for_editor(&app, &cookie, "SI-ISOLATION-FIRST", &["fda"])
			.await?;
	let second_case =
		create_case_for_editor(&app, &cookie, "SI-ISOLATION-SECOND", &["fda"])
			.await?;

	for (case_id, suffix) in [(&first_case, "FIRST"), (&second_case, "SECOND")] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/SI"),
			json!({
				"authorities": ["fda"],
				"rows": {
					"studyInformation": {
						"studyName": format!("Study {suffix}"),
						"fdaCrossReportedIndNumbers": [{
							"indNumber": format!("IND-{suffix}"),
							"sequenceNumber": 1
						}]
					},
					"studyRegistrationNumbers": [{
						"registrationNumber": format!("REG-{suffix}"),
						"countryCode": "US",
						"sequenceNumber": 1
					}]
				}
			}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}

	let first_uri = format!("/api/cases/{first_case}/editor/pages/SI");
	let second_uri = format!("/api/cases/{second_case}/editor/pages/SI");
	let (status, first) = get_json(&app, &cookie, &first_uri).await?;
	assert_eq!(status, StatusCode::OK, "{first}");
	assert_eq!(
		first["rows"]["studyRegistrationNumbers"]
			.as_array()
			.unwrap()
			.len(),
		1
	);
	assert_eq!(
		first["rows"]["studyRegistrationNumbers"][0]["registration_number"],
		"REG-FIRST"
	);
	assert_eq!(
		first["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"]
			.as_array()
			.unwrap()
			.len(),
		1
	);
	assert_eq!(
		first["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"][0]
			["ind_number"],
		"IND-FIRST"
	);

	let (status, second) = get_json(&app, &cookie, &second_uri).await?;
	assert_eq!(status, StatusCode::OK, "{second}");
	let foreign_registration_id = second["rows"]["studyRegistrationNumbers"][0]
		["id"]
		.as_str()
		.ok_or("missing registration id")?;
	let foreign_ind_id = second["rows"]["studyInformation"]
		["fdaCrossReportedIndNumbers"][0]["id"]
		.as_str()
		.ok_or("missing IND id")?;

	for rows in [
		json!({
			"studyRegistrationNumbers": [{
				"id": foreign_registration_id,
				"registrationNumber": "REG-TAMPERED",
				"sequenceNumber": 1
			}]
		}),
		json!({
			"studyRegistrationNumbers": [{
				"id": foreign_registration_id,
				"deleted": true
			}]
		}),
		json!({
			"studyInformation": {
				"fdaCrossReportedIndNumbers": [{
					"id": foreign_ind_id,
					"indNumber": "BAD-IND",
					"sequenceNumber": 1
				}]
			}
		}),
		json!({
			"studyInformation": {
				"fdaCrossReportedIndNumbers": [{
					"id": foreign_ind_id,
					"deleted": true
				}]
			}
		}),
	] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&first_uri,
			json!({"authorities": ["fda"], "rows": rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	}

	let (status, second) = get_json(&app, &cookie, &second_uri).await?;
	assert_eq!(status, StatusCode::OK, "{second}");
	assert_eq!(
		second["rows"]["studyRegistrationNumbers"][0]["registration_number"],
		"REG-SECOND"
	);
	assert_eq!(
		second["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"][0]
			["ind_number"],
		"IND-SECOND"
	);
	Ok(())
}
