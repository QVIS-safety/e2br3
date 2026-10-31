use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
async fn fixture() -> Result<(axum::Router, String, String, String)> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
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
