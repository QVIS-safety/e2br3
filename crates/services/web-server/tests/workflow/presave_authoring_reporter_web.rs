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
async fn test_reporter_presave_imports_into_case_fields_and_persists() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let reporter_data = json!({
		"reporterTitle": "Mr",
		"reporterGivenName": "Reporter",
		"reporterMiddleName": "Middle",
		"reporterFamilyName": "Family",
		"reporterOrganization": "Reporter Org",
		"reporterDepartment": "Dept",
		"reporterStreet": "Street 1",
		"reporterCity": "Busan",
		"reporterState": "Busan",
		"reporterPostcode": "54321",
		"reporterTelephone": "+82-51-1234-5678",
		"reporterCountry": "KR",
		"reporterEmail": "reporter@example.com",
		"qualification": "2",
		"primarySourceForRegulatoryPurposes": "1"
	});
	let (template_id, _) = create_template(
		&app,
		&cookie,
		"reporter",
		"reporter-authoring",
		reporter_data,
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "reporter", &saved_data).await?;

	let (status, reporters) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{reporters:?}");
	let row = reporters["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing reporter row")?;
	assert_eq!(row["reporter_title"].as_str(), Some("Mr"));
	assert_eq!(row["reporter_given_name"].as_str(), Some("Reporter"));
	assert_eq!(row["reporter_middle_name"].as_str(), Some("Middle"));
	assert_eq!(row["reporter_family_name"].as_str(), Some("Family"));
	assert_eq!(row["organization"].as_str(), Some("Reporter Org"));
	assert_eq!(row["department"].as_str(), Some("Dept"));
	assert_eq!(row["street"].as_str(), Some("Street 1"));
	assert_eq!(row["city"].as_str(), Some("Busan"));
	assert_eq!(row["state"].as_str(), Some("Busan"));
	assert_eq!(row["postcode"].as_str(), Some("54321"));
	assert_eq!(row["telephone"].as_str(), Some("+82-51-1234-5678"));
	assert_eq!(row["country_code"].as_str(), Some("KR"));
	assert_eq!(row["email"].as_str(), Some("reporter@example.com"));
	assert_eq!(row["qualification"].as_str(), Some("2"));
	assert_eq!(row["primary_source_regulatory"].as_str(), Some("1"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reporter_presave_imports_minimal_payload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie,
		"reporter",
		"reporter-minimal",
		json!({
			"reporterGivenName": "Minimal",
			"reporterOrganization": "Minimal Reporter Org"
		}),
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "reporter", &saved_data).await?;

	let (status, reporters) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{reporters:?}");
	let row = reporters["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing reporter row")?;
	assert_eq!(row["reporter_given_name"].as_str(), Some("Minimal"));
	assert_eq!(row["organization"].as_str(), Some("Minimal Reporter Org"));
	assert!(row["reporter_family_name"].is_null());
	assert!(row["qualification"].is_null());
	assert!(row["email"].is_null());
	Ok(())
}
