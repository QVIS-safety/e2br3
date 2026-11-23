use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
#[serial]
#[tokio::test]
async fn test_si_legacy_read_projects_persisted_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let case_id =
		create_case_for_editor(&app, &cookie, "LEGACY-SI-READ", &["fda"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/SI");
	let (status,saved)=patch_json(&app,&cookie,&uri,json!({"authorities":["fda"],"rows":{"studyInformation":{"studyName":"Legacy study"},"studyRegistrationNumbers":[{"sequenceNumber":1,"registrationNumber":"LEGACY-REG","countryCode":"US"}]}})).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let row_id = saved["rows"]["studyRegistrationNumbers"][0]["id"].clone();
	let (status, body) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/editor/SI")).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(
		body["data"]["studyInformation"]["study_name"],
		"Legacy study"
	);
	assert_eq!(body["data"]["studyRegistrationNumbers"][0]["id"], row_id);
	assert_eq!(
		body["data"]["studyRegistrationNumbers"][0]["registration_number"],
		"LEGACY-REG"
	);
	assert_eq!(
		body["data"]["studyRegistrationNumbers"][0]["deleted"],
		false
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_si_editor_retains_deleted_children_for_restore_only() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let case_id =
		create_case_for_editor(&app, &cookie, "SI-RESTORE", &["fda"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/SI");
	let (status, saved) = patch_json(&app, &cookie, &uri, json!({
		"authorities": ["fda"], "rows": {
			"studyInformation": {"studyName": "Isolated contract fixture", "fdaCrossReportedIndNumbers": [{"sequenceNumber": 7, "indNumberNullFlavor": "NA"}]},
			"studyRegistrationNumbers": [{"sequenceNumber": 4, "registrationNumber": "Isolated registration fixture", "countryCodeNullFlavor": "NASK"}]
		}
	})).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let study_id = saved["rows"]["studyInformation"]["id"]
		.as_str()
		.expect("study id");
	let registration_id = saved["rows"]["studyRegistrationNumbers"][0]["id"].clone();
	let ind_id = saved["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"][0]
		["id"]
		.clone();
	assert!(registration_id.is_string() && ind_id.is_string());
	for deleted in [true, false] {
		let (status, patched) = patch_json(&app, &cookie, &uri, json!({
			"authorities": ["fda"], "rows": {
				"studyRegistrationNumbers": [{"id": registration_id, "deleted": deleted}],
				"studyInformation": {"fdaCrossReportedIndNumbers": [{"id": ind_id, "deleted": deleted}]}
			}
		})).await?;
		assert_eq!(status, StatusCode::OK, "{patched}");
		for (path, owner) in [
			(&uri, "rows"),
			(&format!("/api/cases/{case_id}/editor/SI"), "data"),
		] {
			let (status, loaded) = get_json(&app, &cookie, path).await?;
			assert_eq!(status, StatusCode::OK, "{loaded}");
			let registration = &loaded[owner]["studyRegistrationNumbers"][0];
			let ind =
				&loaded[owner]["studyInformation"]["fdaCrossReportedIndNumbers"][0];
			assert_eq!(registration["id"], registration_id);
			assert_eq!(ind["id"], ind_id);
			for row in [registration, ind] {
				assert_eq!(row["deleted"], deleted);
				assert_eq!(row["study_information_id"], study_id);
			}
			assert_eq!(registration["sequence_number"], 4);
			assert_eq!(
				registration["registration_number"],
				"Isolated registration fixture"
			);
			assert_eq!(registration["country_code"], serde_json::Value::Null);
			assert_eq!(registration["country_code_null_flavor"], "NASK");
			assert_eq!(ind["sequence_number"], 7);
			assert_eq!(ind["ind_number"], serde_json::Value::Null);
			assert_eq!(ind["ind_number_null_flavor"], "NA");
		}
		for resource in ["registrations", "fda-cross-reported-inds"] {
			let (status, listed) = get_json(&app, &cookie, &format!("/api/cases/{case_id}/safety-report/studies/{study_id}/{resource}")).await?;
			assert_eq!(status, StatusCode::OK, "{listed}");
			assert_eq!(
				listed["data"].as_array().expect("actual list").len(),
				if deleted { 0 } else { 1 }
			);
		}
	}
	Ok(())
}
