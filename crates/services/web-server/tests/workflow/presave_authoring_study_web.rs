use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use crate::presave_authoring::{
	apply_authoring_presave, create_case, create_template, get_template_data,
	request_json,
};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_study_presave_imports_into_case_fields_and_persists() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let study_data = json!({
		"studyName": "Study Alpha",
		"sponsorStudyNumber": "SP-001",
		"studyTypeReaction": "2",
		"studyRegistrationNumber": "REG-001",
		"studyRegistrationCountry": "US"
	});
	let (template_id, _) =
		create_template(&app, &cookie, "study", "study-authoring", study_data)
			.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "study", &saved_data).await?;

	let (status, studies) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/studies"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{studies:?}");
	let study = studies["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing study row")?;
	assert_eq!(study["study_name"].as_str(), Some("Study Alpha"));
	assert_eq!(study["sponsor_study_number"].as_str(), Some("SP-001"));
	assert_eq!(study["study_type_reaction"].as_str(), Some("2"));
	let study_id = study["id"].as_str().ok_or("missing study id")?;

	let (status, regs) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations"
		),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{regs:?}");
	let reg = regs["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing registration row")?;
	assert_eq!(reg["registration_number"].as_str(), Some("REG-001"));
	assert_eq!(reg["country_code"].as_str(), Some("US"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_study_presave_imports_minimal_payload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie,
		"study",
		"study-minimal",
		json!({
			"studyName": "Minimal Study"
		}),
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "study", &saved_data).await?;

	let (status, studies) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/studies"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{studies:?}");
	let study = studies["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing study row")?;
	assert_eq!(study["study_name"].as_str(), Some("Minimal Study"));
	assert!(study["sponsor_study_number"].is_null());
	Ok(())
}
