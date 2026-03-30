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
async fn test_sender_presave_imports_into_case_fields_and_persists() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let sender_data = json!({
		"senderType": "1",
		"senderOrganization": "Sender Presave Org",
		"senderDepartment": "PV Department",
		"senderPersonTitle": "Dr",
		"senderPersonGivenName": "Alice",
		"senderPersonMiddleName": "B",
		"senderPersonFamilyName": "Kim",
		"senderStreetAddress": "1 Safety Street",
		"senderCity": "Seoul",
		"senderState": "Seoul",
		"senderPostcode": "12345",
		"senderCountryCode": "KR",
		"senderTelephone": "+82-2-1111-2222",
		"senderFax": "+82-2-3333-4444",
		"senderEmail": "sender@example.com"
	});
	let (template_id, _) =
		create_template(&app, &cookie, "sender", "sender-authoring", sender_data)
			.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "sender", &saved_data).await?;

	let (status, sender) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/senders"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{sender:?}");
	let row = sender["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing sender row")?;
	assert_eq!(row["sender_type"].as_str(), Some("1"));
	assert_eq!(
		row["organization_name"].as_str(),
		Some("Sender Presave Org")
	);
	assert_eq!(row["department"].as_str(), Some("PV Department"));
	assert_eq!(row["person_title"].as_str(), Some("Dr"));
	assert_eq!(row["person_given_name"].as_str(), Some("Alice"));
	assert_eq!(row["person_middle_name"].as_str(), Some("B"));
	assert_eq!(row["person_family_name"].as_str(), Some("Kim"));
	assert_eq!(row["street_address"].as_str(), Some("1 Safety Street"));
	assert_eq!(row["city"].as_str(), Some("Seoul"));
	assert_eq!(row["state"].as_str(), Some("Seoul"));
	assert_eq!(row["postcode"].as_str(), Some("12345"));
	assert_eq!(row["country_code"].as_str(), Some("KR"));
	assert_eq!(row["telephone"].as_str(), Some("+82-2-1111-2222"));
	assert_eq!(row["fax"].as_str(), Some("+82-2-3333-4444"));
	assert_eq!(row["email"].as_str(), Some("sender@example.com"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_presave_imports_minimal_payload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie,
		"sender",
		"sender-minimal",
		json!({
			"senderType": "2",
			"senderOrganization": "Minimal Sender Org"
		}),
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "sender", &saved_data).await?;

	let (status, sender) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/senders"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{sender:?}");
	let row = sender["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing sender row")?;
	assert_eq!(row["sender_type"].as_str(), Some("2"));
	assert_eq!(
		row["organization_name"].as_str(),
		Some("Minimal Sender Org")
	);
	assert!(row["department"].is_null());
	assert!(row["person_given_name"].is_null());
	assert!(row["email"].is_null());
	Ok(())
}
