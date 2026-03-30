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
async fn test_product_presave_imports_into_case_fields_and_persists() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let product_data = json!({
		"drugCharacterization": "1",
		"medicinalProduct": "Product Alpha",
		"mpidVersion": "MPID-V1",
		"mpid": "MPID-001",
		"phpidVersion": "PHPID-V1",
		"phpid": "PHPID-001",
		"obtainDrugCountry": "US",
		"drugAuthorizationCountry": "US",
		"drugAuthorizationHolder": "Holder Inc",
		"drugAuthorizationNumber": "AUTH-001",
		"drugBrandName": "Brand Alpha",
		"drugGenericName": "Generic Alpha",
		"drugBatchNumber": "BATCH-001",
		"activeSubstances": [{
			"substanceName": "Substance Alpha",
			"substanceTermId": "TERM-001",
			"substanceTermIdVersion": "TERM-V1",
			"substanceStrengthValue": 10.5,
			"substanceStrengthUnit": "mg"
		}]
	});
	let (template_id, _) =
		create_template(&app, &cookie, "product", "product-authoring", product_data)
			.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "product", &saved_data).await?;

	let (status, drugs) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/drugs"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{drugs:?}");
	let drug = drugs["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing drug row")?;
	assert_eq!(drug["drug_characterization"].as_str(), Some("1"));
	assert_eq!(drug["medicinal_product"].as_str(), Some("Product Alpha"));
	assert_eq!(drug["mpid_version"].as_str(), Some("MPID-V1"));
	assert_eq!(drug["mpid"].as_str(), Some("MPID-001"));
	assert_eq!(drug["phpid_version"].as_str(), Some("PHPID-V1"));
	assert_eq!(drug["phpid"].as_str(), Some("PHPID-001"));
	assert_eq!(drug["obtain_drug_country"].as_str(), Some("US"));
	assert_eq!(drug["manufacturer_country"].as_str(), Some("US"));
	assert_eq!(drug["manufacturer_name"].as_str(), Some("Holder Inc"));
	assert_eq!(drug["drug_authorization_number"].as_str(), Some("AUTH-001"));
	assert_eq!(drug["brand_name"].as_str(), Some("Brand Alpha"));
	assert_eq!(drug["drug_generic_name"].as_str(), Some("Generic Alpha"));
	assert_eq!(drug["batch_lot_number"].as_str(), Some("BATCH-001"));
	let drug_id = drug["id"].as_str().ok_or("missing drug id")?;

	let (status, substances) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{substances:?}");
	let substance = substances["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing substance row")?;
	assert_eq!(
		substance["substance_name"].as_str(),
		Some("Substance Alpha")
	);
	assert_eq!(substance["substance_termid"].as_str(), Some("TERM-001"));
	assert_eq!(
		substance["substance_termid_version"].as_str(),
		Some("TERM-V1")
	);
	assert_eq!(substance["strength_unit"].as_str(), Some("mg"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_product_presave_imports_minimal_payload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie,
		"product",
		"product-minimal",
		json!({
			"drugCharacterization": "2",
			"medicinalProduct": "Minimal Product"
		}),
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "product", &saved_data).await?;

	let (status, drugs) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/drugs"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{drugs:?}");
	let drug = drugs["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing drug row")?;
	assert_eq!(drug["drug_characterization"].as_str(), Some("2"));
	assert_eq!(drug["medicinal_product"].as_str(), Some("Minimal Product"));
	assert!(drug["drug_generic_name"].is_null());
	let drug_id = drug["id"].as_str().ok_or("missing drug id")?;

	let (status, substances) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{substances:?}");
	assert_eq!(
		substances["data"].as_array().map(|rows| rows.len()),
		Some(0)
	);
	Ok(())
}
