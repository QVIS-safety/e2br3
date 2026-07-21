use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
async fn test_canonical_product_presave_is_authorityless_union_record() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;

	let created = post_json_created(
		&app,
		&admin_cookie,
		"/api/presaves/products".to_string(),
		json!({
			"data": {
				"sender_presave_id": sender_id,
				"product_id": "UNION-PRODUCT",
				"medicinal_product": "Union Product"
			}
		}),
	)
	.await?;
	assert!(
		created["data"].get("authority").is_none(),
		"canonical presave responses must not expose authority: {created:?}"
	);
	let product_id = data_id(&created)?;

	let saved = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
	)
	.await?;
	assert!(saved["data"]["parent"]
		.get("unknown_extra_product_code")
		.is_none());
	assert_eq!(
		saved["data"].get("mfds_device_items"),
		None,
		"Product Presave details must not expose MFDS device rows"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn product_presave_details_expose_effective_mfds_dg_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "mfds").await?;

	let created = post_json_created(
		&app,
		&admin_cookie,
		"/api/presaves/products".to_string(),
		json!({
			"data": {
				"sender_presave_id": sender_id,
				"product_id": "EFFECTIVE-MFDS-PRODUCT",
				"medicinal_product": "Effective MFDS Product",
				"mfds_mpid": "KR-MPID",
				"mfds_mpid_version": "KR-V1"
			}
		}),
	)
	.await?;
	let product_id = data_id(&created)?;

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
		json!({
			"data": {
				"active_substances": [
					{
						"sequence_number": 1,
						"substance_name": "Acetaminophen",
						"substance_termid_version": "ICH-SUB-V1",
						"substance_termid": "ICH-SUB",
						"mfds_version": "KR-SUB-V1",
						"mfds_id": "KR-SUB",
						"strength_value": "500",
						"strength_unit": "mg"
					}
				]
			}
		}),
	)
	.await?;

	assert_eq!(
		saved["data"]["parent"]["mfds_mpid"].as_str(),
		Some("KR-MPID")
	);
	assert_eq!(
		saved["data"]["parent"]["mfds_mpid_version"].as_str(),
		Some("KR-V1")
	);
	assert!(saved["data"]["parent"]
		.get("unknown_extra_product_code")
		.is_none());
	let substance = &saved["data"]["active_substances"]
		.as_array()
		.ok_or("missing active_substances")?[0];
	assert_eq!(substance["mfds_id"].as_str(), Some("KR-SUB"));
	assert_eq!(substance["mfds_version"].as_str(), Some("KR-SUB-V1"));
	assert_eq!(substance["substance_termid"].as_str(), Some("ICH-SUB"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_product_presave_rejects_missing_sender_or_identity() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;

	for (body, label) in [
		(
			json!({
				"data": {
					"product_id": format!("PRODUCT-{}", Uuid::new_v4())
				}
			}),
			"missing sender",
		),
		(
			json!({
				"data": {
					"sender_presave_id": sender_id,
					"product_id": " ",
					"preapproval_ip_name": " "
				}
			}),
			"missing product identity",
		),
	] {
		let (status, value) = request_json(
			&app,
			&admin_cookie,
			Method::POST,
			"/api/presaves/products".to_string(),
			Some(body),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
		assert!(
			value
				.to_string()
				.contains("product presave requires sender_presave_id and product_id or preapproval_ip_name"),
			"unexpected product validation body for {label}: {value:?}"
		);
	}

	let product_id = create_product_presave_with_identity_for_sender_via_api(
		&app,
		&admin_cookie,
		sender_id,
		Some(&format!("PRODUCT-{}", Uuid::new_v4())),
		None,
	)
	.await?;
	for (method, uri, body, label) in [(
		Method::PUT,
		format!("/api/presaves/products/{product_id}/details"),
		json!({ "data": { "parent": { "product_id": " ", "preapproval_ip_name": " " } } }),
		"details missing identity",
	)] {
		let (status, value) =
			request_json(&app, &admin_cookie, method, uri, Some(body)).await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
		assert!(
			value
				.to_string()
				.contains("product presave requires sender_presave_id and product_id or preapproval_ip_name"),
			"unexpected product validation body for {label}: {value:?}"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_product_presave_rejects_duplicate_identity_under_same_sender(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;
	let other_sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;
	let product_id_value = format!("DUP-PRODUCT-{}", Uuid::new_v4());
	let ip_name_value = format!("DUP-IP-{}", Uuid::new_v4());

	let first_id = create_product_presave_with_identity_for_sender_via_api(
		&app,
		&admin_cookie,
		sender_id,
		Some(&product_id_value),
		Some(&ip_name_value),
	)
	.await?;

	for (body, label) in [
		(
			json!({
				"data": {
					"sender_presave_id": sender_id,
					"product_id": product_id_value.clone(),
					"medicinal_product": "Duplicate Product ID"
				}
			}),
			"product_id",
		),
		(
			json!({
				"data": {
					"sender_presave_id": sender_id,
					"preapproval_ip_name": ip_name_value.clone(),
					"medicinal_product": "Duplicate IP Name"
				}
			}),
			"preapproval_ip_name",
		),
	] {
		let (status, value) = request_json(
			&app,
			&admin_cookie,
			Method::POST,
			"/api/presaves/products".to_string(),
			Some(body),
		)
		.await?;
		assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
		assert!(
			value
				.to_string()
				.contains("product presave duplicate identity"),
			"unexpected duplicate product {label} body: {value:?}"
		);
	}

	let reused_by_other_sender =
		create_product_presave_with_identity_for_sender_via_api(
			&app,
			&admin_cookie,
			other_sender_id,
			Some(&product_id_value),
			Some(&ip_name_value),
		)
		.await?;
	assert_ne!(first_id, reused_by_other_sender);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_product_presave_details_graph_load_and_save() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let product_id =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
	let substance_id = create_product_active_substance_via_api(
		&app,
		&admin_cookie,
		product_id,
		1,
		"Substance A",
	)
	.await?;
	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], product_id.to_string());
	assert_eq!(
		details["data"]["active_substances"][0]["id"],
		substance_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
		json!({
			"data": {
				"parent": { "brand_name": "Graph Brand" },
				"active_substances": [
					{
						"id": substance_id,
						"sequence_number": 2,
						"substance_name": "Substance Updated",
						"strength_value": "7.5",
						"strength_unit": "mg"
					},
					{
						"sequence_number": 3,
						"substance_name": "Substance Created"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(saved["data"]["parent"]["brand_name"], "Graph Brand");
	assert_eq!(
		saved["data"]["active_substances"].as_array().unwrap().len(),
		2
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn product_presave_details_hides_old_source_fields_and_excludes_mfds_device_items(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let product_id =
		create_product_presave_via_api(&app, &admin_cookie, "mfds").await?;

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
		json!({
			"data": {
				"parent": {
					"mfds_mpid": "KR-MPID",
					"mfds_mpid_version": "KR-V1"
				}
			}
		}),
	)
	.await?;
	assert_eq!(saved["data"]["parent"]["mfds_mpid"], "KR-MPID");
	assert!(saved["data"]["parent"]
		.get("unknown_extra_product_code")
		.is_none());
	assert!(saved["data"]["parent"]
		.get("unknown_extra_udl_product_code")
		.is_none());
	assert!(saved["data"]["parent"]
		.get("unknown_extra_foreign_ich_product_code")
		.is_none());
	assert!(saved["data"]["parent"]
		.get("unknown_extra_foreign_e2b_product_code")
		.is_none());
	assert_eq!(
		saved["data"].get("mfds_device_items"),
		None,
		"Product Presave details must not expose MFDS device rows"
	);

	let loaded = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
	)
	.await?;
	assert_eq!(
		loaded["data"].get("mfds_device_items"),
		None,
		"Product Presave details must not expose MFDS device rows"
	);
	assert!(loaded["data"]["parent"]
		.get("unknown_extra_product_code")
		.is_none());

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_product_presave_details_noop_delete_and_invalid_child_operations(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let product_a =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
	let product_b =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
	let substance_delete = create_product_active_substance_via_api(
		&app,
		&admin_cookie,
		product_a,
		1,
		"Delete Substance",
	)
	.await?;
	let substance_keep = create_product_active_substance_via_api(
		&app,
		&admin_cookie,
		product_a,
		2,
		"Keep Substance",
	)
	.await?;
	let wrong_parent_substance = create_product_active_substance_via_api(
		&app,
		&admin_cookie,
		product_b,
		1,
		"Other Product Substance",
	)
	.await?;
	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
		json!({ "data": { "parent": { "brand_name": "Product Noop" } } }),
	)
	.await?;
	let after_omit = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
	)
	.await?;
	assert_eq!(
		after_omit["data"]["active_substances"]
			.as_array()
			.unwrap()
			.len(),
		2
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
		json!({
			"data": {
				"active_substances": []
			}
		}),
	)
	.await?;
	let after_empty = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
	)
	.await?;
	assert_eq!(
		after_empty["data"]["active_substances"]
			.as_array()
			.unwrap()
			.len(),
		2
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
		json!({
			"data": {
				"active_substances": [{ "id": substance_delete, "_delete": true }]
			}
		}),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
	)
	.await?;
	let active_substances = after_delete["data"]["active_substances"]
		.as_array()
		.unwrap();
	assert!(
		!active_substances
			.iter()
			.any(|row| row["id"].as_str() == Some(&substance_delete.to_string())),
		"{after_delete:?}"
	);
	assert!(
		active_substances
			.iter()
			.any(|row| row["id"].as_str() == Some(&substance_keep.to_string())),
		"{after_delete:?}"
	);
	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/products/{product_a}/details"),
		Some(json!({ "data": { "active_substances": [{ "_delete": true }] } })),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/products/{product_a}/details"),
		Some(json!({
			"data": {
				"active_substances": [{
					"id": wrong_parent_substance,
					"sequence_number": 2,
					"substance_name": "Wrong Parent"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/products/{product_a}/details"),
		Some(json!({ "data": { "substances": [] } })),
	)
	.await?;
	assert!(
		status.is_client_error(),
		"legacy Product key was accepted: {value:?}"
	);

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
	)
	.await?;
	let sender_id = details["data"]["parent"]["sender_presave_id"].clone();
	for (method, uri, body) in [
		(
			Method::POST,
			"/api/presaves/products".to_string(),
			json!({
				"data": {
					"sender_presave_id": sender_id,
					"product_id": format!("LEGACY-{}", Uuid::new_v4()),
					"mpid_version_date_number": "legacy"
				}
			}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/products/{product_a}"),
			json!({ "data": { "phpid_version_date_number": "legacy" } }),
		),
		(
			Method::POST,
			format!("/api/presaves/products/{product_a}/active-substances"),
			json!({ "data": { "sequence_number": 3, "name": "legacy" } }),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/products/{product_a}/active-substances/{substance_keep}"
			),
			json!({ "data": { "strength_number": "1" } }),
		),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, method, uri, Some(body)).await?;
		assert!(
			status.is_client_error(),
			"legacy Product payload was accepted: {value:?}"
		);
	}

	Ok(())
}
