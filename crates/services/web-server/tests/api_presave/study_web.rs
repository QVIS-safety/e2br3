use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn study_presave_create_persists_all_canonical_rows_atomically() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let product_id = create_product_presave(&mm, seed.org_id, seed.admin.id).await?;
	let reporter_id = create_named_reporter_presave_via_api(
		&app,
		&cookie,
		format!("Study Rows Reporter {}", Uuid::new_v4()),
		"Rows Reporter Org",
	)
	.await?;

	let created = post_json_created(
		&app, &cookie, "/api/presaves/studies".to_string(),
		json!({ "data": { "rows": {
			"study": {
				"productPresaveId": product_id,
				"studyName": "Atomic Rows Study",
				"sponsorStudyNumber": format!("ROWS-{}", Uuid::new_v4()),
				"studyTypeReaction": "1"
			},
			"products": [{ "sequenceNumber": 1, "productPresaveId": product_id, "productName": "Rows Product", "deleted": false }],
			"reporters": [{ "sequenceNumber": 1, "reporterPresaveId": reporter_id, "reporterOrganization": "Rows Reporter Org", "deleted": false }],
			"registrationNumbers": [{ "sequenceNumber": 1, "registrationNumber": "ROWS-REG", "countryCode": "US", "deleted": false }],
			"fdaCrossReportedInds": [{ "sequenceNumber": 1, "indNumber": "IND-ROWS", "deleted": false }]
		} } }),
	).await?;

	assert_eq!(
		created["data"]["rows"]["study"]["studyName"],
		"Atomic Rows Study"
	);
	assert_eq!(
		created["data"]["rows"]["products"][0]["productName"],
		"Rows Product"
	);
	assert_eq!(
		created["data"]["rows"]["reporters"][0]["reporterOrganization"],
		"Rows Reporter Org"
	);
	assert_eq!(
		created["data"]["rows"]["registrationNumbers"][0]["registrationNumber"],
		"ROWS-REG"
	);
	assert_eq!(
		created["data"]["rows"]["fdaCrossReportedInds"][0]["indNumber"],
		"IND-ROWS"
	);
	Ok(())
}

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
	assert_eq!(details["data"]["rows"]["study"]["id"], study_id.to_string());
	assert_eq!(
		details["data"]["rows"]["registrationNumbers"][0]["id"],
		registration_id.to_string()
	);
	assert_eq!(
		details["data"]["rows"]["products"][0]["id"],
		study_product_id.to_string()
	);
	assert_eq!(
		details["data"]["rows"]["reporters"][0]["id"],
		study_reporter_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({
			"data": { "rows": {
				"study": { "studyName": "Study Graph Updated" },
				"registrationNumbers": [
					{
						"id": registration_id,
						"sequenceNumber": 2,
						"registrationNumber": "REG-UPDATED",
						"countryCode": "CA"
					},
					{
						"sequenceNumber": 3,
						"registrationNumber": "REG-CREATED",
						"countryCode": "US"
					}
				],
				"products": [
					{ "id": study_product_id, "sequenceNumber": 2, "productPresaveId": product_id, "productName": "Study Product Updated" },
					{ "sequenceNumber": 3, "productPresaveId": product_id, "productName": "Study Product Created" }
				],
				"reporters": [
					{ "id": study_reporter_id, "sequenceNumber": 2, "reporterPresaveId": reporter_id, "reporterOrganization": "Study Reporter Updated" },
					{ "sequenceNumber": 3, "reporterPresaveId": reporter_id, "reporterOrganization": "Study Reporter Created" }
				]
			} }
		}),
	)
	.await?;
	assert_eq!(
		saved["data"]["rows"]["study"]["studyName"],
		"Study Graph Updated"
	);
	assert_eq!(
		saved["data"]["rows"]["registrationNumbers"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		saved["data"]["rows"]["products"].as_array().unwrap().len(),
		2
	);
	assert_eq!(
		saved["data"]["rows"]["reporters"].as_array().unwrap().len(),
		2
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({
			"data": { "rows": {
				"registrationNumbers": [{ "id": registration_id, "deleted": true }]
			} }
		}),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	let deleted_registration = after_delete["data"]["rows"]["registrationNumbers"]
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
	assert_eq!(details["data"]["rows"]["study"]["id"], study_id.to_string());
	assert_eq!(
		details["data"]["rows"]["registrationNumbers"][0]["id"],
		registration_id.to_string()
	);
	assert_eq!(
		details["data"]["rows"]["products"][0]["id"],
		study_product_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({
			"data": { "rows": {
				"study": { "studyName": "updated by study graph" },
				"registrationNumbers": [
					{
						"id": registration_id,
						"sequenceNumber": 2,
						"registrationNumber": "REG-2",
						"countryCode": "CA"
					},
					{
						"sequenceNumber": 3,
						"registrationNumber": "REG-3",
						"countryCode": "GB"
					}
				],
				"products": [
					{
						"id": study_product_id,
						"sequenceNumber": 2,
						"productPresaveId": product_id,
						"productName": "Study Product 2"
					},
					{
						"sequenceNumber": 3,
						"productPresaveId": product_id,
						"productName": "Study Product 3"
					}
				]
			} }
		}),
	)
	.await?;
	assert!(
		saved["data"]["rows"]["study"].get("comments").is_none(),
		"{saved:?}"
	);
	assert_eq!(
		saved["data"]["rows"]["study"]["studyName"].as_str(),
		Some("updated by study graph"),
		"{saved:?}"
	);
	assert_eq!(
		saved["data"]["rows"]["registrationNumbers"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		saved["data"]["rows"]["products"].as_array().unwrap().len(),
		2
	);

	let persisted = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	let registrations = persisted["data"]["rows"]["registrationNumbers"]
		.as_array()
		.unwrap();
	let updated_registration = registrations
		.iter()
		.find(|row| row["id"].as_str() == Some(&registration_id.to_string()))
		.ok_or("missing updated registration")?;
	assert_eq!(
		updated_registration["registrationNumber"].as_str(),
		Some("REG-2")
	);
	assert_eq!(updated_registration["countryCode"].as_str(), Some("CA"));
	let created_registration = registrations
		.iter()
		.find(|row| row["registrationNumber"].as_str() == Some("REG-3"))
		.ok_or("missing created registration")?;
	assert_eq!(created_registration["countryCode"].as_str(), Some("GB"));

	let products = persisted["data"]["rows"]["products"].as_array().unwrap();
	assert!(
		products
			.iter()
			.any(|row| row["productName"].as_str() == Some("Study Product 2")),
		"{persisted:?}"
	);
	assert!(
		products
			.iter()
			.any(|row| row["productName"].as_str() == Some("Study Product 3")),
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
		json!({ "data": { "rows": { "study": { "studyName": "omit children" } } } }),
	)
	.await?;
	let after_omit = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	assert_eq!(
		after_omit["data"]["rows"]["registrationNumbers"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		after_omit["data"]["rows"]["products"]
			.as_array()
			.unwrap()
			.len(),
		1
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({ "data": { "rows": { "registrationNumbers": [], "products": [] } } }),
	)
	.await?;
	let after_empty = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
	)
	.await?;
	assert_eq!(
		after_empty["data"]["rows"]["registrationNumbers"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		after_empty["data"]["rows"]["products"]
			.as_array()
			.unwrap()
			.len(),
		1
	);

	let after_delete = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({
			"data": { "rows": {
				"registrationNumbers": [{ "id": registration_delete_id, "deleted": true }]
			} }
		}),
	)
	.await?;
	let registrations = after_delete["data"]["rows"]["registrationNumbers"]
		.as_array()
		.unwrap();
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
		after_delete["data"]["rows"]["products"]
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
		json!({ "data": { "rows": { "registrationNumbers": [{ "deleted": true }] } } }),
		json!({ "data": { "rows": { "products": [{ "deleted": true }] } } }),
		json!({ "data": { "rows": { "registrationNumbers": [{ "id": registration_b, "deleted": true }] } } }),
		json!({ "data": { "rows": { "products": [{ "id": product_b_child, "deleted": true }] } } }),
		json!({ "data": { "rows": { "registrationNumbers": [{ "id": registration_b, "sequenceNumber": 2, "registrationNumber": "WRONG" }] } } }),
		json!({ "data": { "rows": { "products": [{ "id": product_b_child, "sequenceNumber": 2, "productName": "WRONG" }] } } }),
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

	for legacy_body in [
		json!({ "data": { "parent": {} } }),
		json!({ "data": { "registration_numbers": [] } }),
		json!({ "data": { "rows": { "fda_cross_reported_inds": [] } } }),
	] {
		let (status, value) = request_json(
			&app,
			&admin_cookie,
			Method::PUT,
			format!("/api/presaves/studies/{study_a}/details"),
			Some(legacy_body),
		)
		.await?;
		assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{value:?}");
	}

	Ok(())
}
