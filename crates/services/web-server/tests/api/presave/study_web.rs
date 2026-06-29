use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_study_presave_details_graph_load_save_and_delete() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let product_id =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
	let study_id = create_study_presave_for_product_via_api(
		&app,
		&admin_cookie,
		product_id,
		"fda",
	)
	.await?;
	let registration_id = create_study_registration_number_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		"REG-OLD",
	)
	.await?;
	let reporter_id = create_named_reporter_presave_via_api(
		&app,
		&admin_cookie,
		format!("REST Study Reporter {}", Uuid::new_v4()),
		"Study Reporter Org",
	)
	.await?;
	let study_product_id = create_study_product_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		product_id,
		"Study Product Old",
	)
	.await?;
	let study_reporter_id = create_study_reporter_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		reporter_id,
		"Study Reporter Org",
	)
	.await?;

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], study_id.to_string());
	assert_eq!(
		details["data"]["registrations"][0]["id"],
		registration_id.to_string()
	);
	assert_eq!(
		details["data"]["products"][0]["id"],
		study_product_id.to_string()
	);
	assert_eq!(
		details["data"]["reporters"][0]["id"],
		study_reporter_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({
			"data": {
				"parent": { "study_name": "Study Graph Updated" },
				"registrations": [
					{
						"id": registration_id,
						"sequence_number": 2,
						"registration_number": "REG-UPDATED",
						"country_code": "CA"
					},
					{
						"sequence_number": 3,
						"registration_number": "REG-CREATED",
						"country_code": "US"
					}
				],
				"products": [
					{ "id": study_product_id, "sequence_number": 2, "product_presave_id": product_id, "product_name": "Study Product Updated" },
					{ "sequence_number": 3, "product_presave_id": product_id, "product_name": "Study Product Created" }
				],
				"reporters": [
					{ "id": study_reporter_id, "sequence_number": 2, "reporter_presave_id": reporter_id, "reporter_organization": "Study Reporter Updated" },
					{ "sequence_number": 3, "reporter_presave_id": reporter_id, "reporter_organization": "Study Reporter Created" }
				]
			}
		}),
	)
	.await?;
	assert_eq!(saved["data"]["parent"]["study_name"], "Study Graph Updated");
	assert_eq!(saved["data"]["registrations"].as_array().unwrap().len(), 2);
	assert_eq!(saved["data"]["products"].as_array().unwrap().len(), 2);
	assert_eq!(saved["data"]["reporters"].as_array().unwrap().len(), 2);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({
			"data": {
				"registrations": [{ "id": registration_id, "_delete": true }]
			}
		}),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	let deleted_registration = after_delete["data"]["registrations"]
		.as_array()
		.unwrap()
		.iter()
		.find(|row| row["id"].as_str() == Some(&registration_id.to_string()))
		.ok_or("missing deleted registration")?
		.clone();
	assert_eq!(deleted_registration["deleted"].as_bool(), Some(true));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_study_presave_details_graph_load_and_save() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let product_id = create_product_presave(&mm, seed.org_id, seed.admin.id).await?;
	let study_id = create_study_presave_for_product_via_api(
		&app,
		&admin_cookie,
		product_id,
		"fda",
	)
	.await?;

	let registration_id = create_study_registration_number_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		"REG-1",
	)
	.await?;
	let study_product_id = create_study_product_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		product_id,
		"Study Product 1",
	)
	.await?;

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], study_id.to_string());
	assert_eq!(
		details["data"]["registrations"][0]["id"],
		registration_id.to_string()
	);
	assert_eq!(
		details["data"]["products"][0]["id"],
		study_product_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({
			"data": {
				"parent": { "comments": "updated by study graph" },
				"registrations": [
					{
						"id": registration_id,
						"sequence_number": 2,
						"registration_number": "REG-2",
						"country_code": "CA"
					},
					{
						"sequence_number": 3,
						"registration_number": "REG-3",
						"country_code": "GB"
					}
				],
				"products": [
					{
						"id": study_product_id,
						"sequence_number": 2,
						"product_presave_id": product_id,
						"product_name": "Study Product 2"
					},
					{
						"sequence_number": 3,
						"product_presave_id": product_id,
						"product_name": "Study Product 3"
					}
				]
			}
		}),
	)
	.await?;
	assert!(saved["data"]["parent"]["comments"].is_null(), "{saved:?}");
	assert_eq!(saved["data"]["registrations"].as_array().unwrap().len(), 2);
	assert_eq!(saved["data"]["products"].as_array().unwrap().len(), 2);

	let persisted = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	let registrations = persisted["data"]["registrations"].as_array().unwrap();
	let updated_registration = registrations
		.iter()
		.find(|row| row["id"].as_str() == Some(&registration_id.to_string()))
		.ok_or("missing updated registration")?;
	assert_eq!(
		updated_registration["registration_number"].as_str(),
		Some("REG-2")
	);
	assert_eq!(updated_registration["country_code"].as_str(), Some("CA"));
	let created_registration = registrations
		.iter()
		.find(|row| row["registration_number"].as_str() == Some("REG-3"))
		.ok_or("missing created registration")?;
	assert_eq!(created_registration["country_code"].as_str(), Some("GB"));

	let products = persisted["data"]["products"].as_array().unwrap();
	assert!(
		products
			.iter()
			.any(|row| row["product_name"].as_str() == Some("Study Product 2")),
		"{persisted:?}"
	);
	assert!(
		products
			.iter()
			.any(|row| row["product_name"].as_str() == Some("Study Product 3")),
		"{persisted:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_study_presave_details_requires_explicit_child_delete() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let product_id = create_product_presave(&mm, seed.org_id, seed.admin.id).await?;
	let study_id = create_study_presave_for_product_via_api(
		&app,
		&admin_cookie,
		product_id,
		"fda",
	)
	.await?;
	let registration_delete_id = create_study_registration_number_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		"DELETE",
	)
	.await?;
	let registration_keep_id = create_study_registration_number_via_api(
		&app,
		&admin_cookie,
		study_id,
		2,
		"KEEP",
	)
	.await?;
	let study_product_id = create_study_product_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		product_id,
		"KEEP-PRODUCT",
	)
	.await?;

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({ "data": { "parent": { "comments": "omit children" } } }),
	)
	.await?;
	let after_omit = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	assert_eq!(
		after_omit["data"]["registrations"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(after_omit["data"]["products"].as_array().unwrap().len(), 1);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({ "data": { "registrations": [], "products": [] } }),
	)
	.await?;
	let after_empty = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	assert_eq!(
		after_empty["data"]["registrations"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(after_empty["data"]["products"].as_array().unwrap().len(), 1);

	let after_delete = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({
			"data": {
				"registrations": [{ "id": registration_delete_id, "_delete": true }]
			}
		}),
	)
	.await?;
	let registrations = after_delete["data"]["registrations"].as_array().unwrap();
	let deleted_registration = registrations
		.iter()
		.find(|row| row["id"].as_str() == Some(&registration_delete_id.to_string()))
		.ok_or("missing deleted registration")?;
	assert_eq!(deleted_registration["deleted"].as_bool(), Some(true));
	let kept_registration = registrations
		.iter()
		.find(|row| row["id"].as_str() == Some(&registration_keep_id.to_string()))
		.ok_or("missing kept registration")?;
	assert_eq!(kept_registration["deleted"].as_bool(), Some(false));
	assert!(
		after_delete["data"]["products"]
			.as_array()
			.unwrap()
			.iter()
			.any(|row| row["id"].as_str() == Some(&study_product_id.to_string())),
		"{after_delete:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_study_presave_details_rejects_invalid_child_operations() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let product_a = create_product_presave(&mm, seed.org_id, seed.admin.id).await?;
	let product_b = create_product_presave(&mm, seed.org_id, seed.admin.id).await?;
	let study_a = create_study_presave_for_product_via_api(
		&app,
		&admin_cookie,
		product_a,
		"fda",
	)
	.await?;
	let study_b = create_study_presave_for_product_via_api(
		&app,
		&admin_cookie,
		product_b,
		"fda",
	)
	.await?;
	let registration_b = create_study_registration_number_via_api(
		&app,
		&admin_cookie,
		study_b,
		1,
		"OTHER",
	)
	.await?;
	let product_b_child = create_study_product_via_api(
		&app,
		&admin_cookie,
		study_b,
		1,
		product_b,
		"OTHER-PRODUCT",
	)
	.await?;

	for body in [
		json!({ "data": { "registrations": [{ "_delete": true }] } }),
		json!({ "data": { "products": [{ "_delete": true }] } }),
		json!({ "data": { "registrations": [{ "id": registration_b, "_delete": true }] } }),
		json!({ "data": { "products": [{ "id": product_b_child, "_delete": true }] } }),
		json!({ "data": { "registrations": [{ "id": registration_b, "sequence_number": 2, "registration_number": "WRONG" }] } }),
		json!({ "data": { "products": [{ "id": product_b_child, "sequence_number": 2, "product_name": "WRONG" }] } }),
	] {
		let (status, value) = request_json(
			&app,
			&admin_cookie,
			Method::PUT,
			format!("/api/presaves/studies/{study_a}/details"),
			Some(body),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	}

	Ok(())
}
