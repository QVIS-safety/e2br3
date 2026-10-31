use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn product_presave_details_accept_case_editor_rows_contract() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let product_id =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
		json!({
			"data": {
				"rows": {
					"product": { "medicinalProduct": "Rows Product" },
					"activeSubstances": [{
						"sequenceNumber": 1,
						"substanceName": "Caffeine",
						"substanceStrengthValue": "10",
						"substanceStrengthUnit": "mg",
						"deleted": false
					}]
				}
			}
		}),
	)
	.await?;

	assert_eq!(
		saved["data"]["rows"]["product"]["medicinalProduct"],
		"Rows Product"
	);
	assert_eq!(
		saved["data"]["rows"]["activeSubstances"][0]["substanceName"],
		"Caffeine"
	);
	assert_eq!(
		saved["data"]["rows"]["activeSubstances"][0]["deleted"],
		false
	);

	for legacy_data in [
		json!({ "parent": {} }),
		json!({ "active_substances": [] }),
		json!({ "substances": [] }),
		json!({ "changes": [] }),
		json!({ "rows": { "activeSubstances": [{ "id": saved["data"]["rows"]["activeSubstances"][0]["id"], "_delete": true }] } }),
		json!({ "rows": { "product": { "sender": "display only" } } }),
		json!({ "rows": { "product": { "receiver": "display only" } } }),
	] {
		let (status, value) = request_json(
			&app,
			&admin_cookie,
			Method::PUT,
			format!("/api/presaves/products/{product_id}/details"),
			Some(json!({ "data": legacy_data })),
		)
		.await?;
		assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{value:?}");
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn product_presave_create_persists_complete_rows_atomically() -> Result<()> {
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
			"data": { "rows": {
				"product": {
					"senderPresaveId": sender_id,
					"productId": format!("ROWS-{}", Uuid::new_v4()),
					"medicinalProduct": "Atomic Rows Product"
				},
				"activeSubstances": [{
					"sequenceNumber": 1,
					"substanceName": "Caffeine",
					"substanceStrengthValue": "10",
					"substanceStrengthUnit": "mg",
					"deleted": false
				}]
			} }
		}),
	)
	.await?;

	assert_eq!(
		created["data"]["rows"]["product"]["medicinalProduct"],
		"Atomic Rows Product"
	);
	assert_eq!(
		created["data"]["rows"]["activeSubstances"][0]["substanceName"],
		"Caffeine"
	);
	assert!(created["data"]["rows"]["activeSubstances"][0]["id"].is_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn info_editor_can_create_and_list_only_products_in_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (editor_id, editor_cookie) =
		create_info_editor(&app, &mm, &admin_cookie, seed.org_id).await?;
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;

	let allowed = post_json_created(
		&app,
		&editor_cookie,
		"/api/presaves/products".to_string(),
		json!({ "data": { "rows": {
			"product": {
				"senderPresaveId": sender_id,
				"productId": format!("INFO-EDITOR-{}", Uuid::new_v4())
			},
			"activeSubstances": []
		} } }),
	)
	.await?;
	let blocked_sender =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;
	let blocked = post_json_created(
		&app,
		&admin_cookie,
		"/api/presaves/products".to_string(),
		json!({ "data": { "rows": {
				"product": {
					"senderPresaveId": blocked_sender,
					"productId": "INFO-EDITOR-BLOCKED"
				},
				"activeSubstances": []
			} } }),
	)
	.await?;
	let allowed_id = data_rows_id(&allowed, "product")?;
	let blocked_id = data_rows_id(&blocked, "product")?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/users/{editor_id}"),
		Some(json!({ "data": {
			"access_sender_ids": [sender_id.to_string()]
		} })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let listed =
		get_json_ok(&app, &editor_cookie, "/api/presaves/products".to_string())
			.await?;
	let ids: Vec<_> = listed["data"]
		.as_array()
		.ok_or("missing product list")?
		.iter()
		.filter_map(|product| product["id"].as_str())
		.collect();
	assert!(ids.contains(&allowed_id.to_string().as_str()));
	assert!(!ids.contains(&blocked_id.to_string().as_str()));
	Ok(())
}

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
		json!({ "data": { "rows": {
			"product": {
				"senderPresaveId": sender_id,
				"productId": "UNION-PRODUCT",
				"medicinalProduct": "Union Product"
			},
			"activeSubstances": []
		} } }),
	)
	.await?;
	assert!(
		created["data"].get("authority").is_none(),
		"canonical presave responses must not expose authority: {created:?}"
	);
	let product_id = data_rows_id(&created, "product")?;

	let saved = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
	)
	.await?;
	assert!(saved["data"]["rows"]["product"]
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
		json!({ "data": { "rows": {
			"product": {
				"senderPresaveId": sender_id,
				"productId": "EFFECTIVE-MFDS-PRODUCT",
				"medicinalProduct": "Effective MFDS Product",
				"mfdsMpid": "KR-MPID",
				"mfdsMpidVersion": "KR-V1"
			},
			"activeSubstances": []
		} } }),
	)
	.await?;
	let product_id = data_rows_id(&created, "product")?;

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
		json!({ "data": { "rows": {
				"activeSubstances": [
					{
						"sequenceNumber": 1,
						"substanceName": "Acetaminophen",
						"substanceTermIdVersion": "ICH-SUB-V1",
						"substanceTermId": "ICH-SUB",
						"mfdsVersion": "KR-SUB-V1",
						"mfdsId": "KR-SUB",
						"substanceStrengthValue": "500",
						"substanceStrengthUnit": "mg"
					}
				], "product": {}
		} } }),
	)
	.await?;

	assert_eq!(
		saved["data"]["rows"]["product"]["mfdsMpid"].as_str(),
		Some("KR-MPID")
	);
	assert_eq!(
		saved["data"]["rows"]["product"]["mfdsMpidVersion"].as_str(),
		Some("KR-V1")
	);
	assert!(saved["data"]["rows"]["product"]
		.get("unknown_extra_product_code")
		.is_none());
	let substance = &saved["data"]["rows"]["activeSubstances"]
		.as_array()
		.ok_or("missing active_substances")?[0];
	assert_eq!(substance["mfdsId"].as_str(), Some("KR-SUB"));
	assert_eq!(substance["mfdsVersion"].as_str(), Some("KR-SUB-V1"));
	assert_eq!(substance["substanceTermId"].as_str(), Some("ICH-SUB"));

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
			json!({ "data": { "rows": {
				"product": { "productId": format!("PRODUCT-{}", Uuid::new_v4()) },
				"activeSubstances": []
			} } }),
			"missing sender",
		),
		(
			json!({ "data": { "rows": {
				"product": {
					"senderPresaveId": sender_id,
					"productId": " ",
					"preApprovalIpName": " "
				},
				"activeSubstances": []
			} } }),
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
		json!({ "data": { "rows": { "product": {
			"productId": " ", "preApprovalIpName": " "
		} } } }),
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
			json!({ "data": { "rows": {
				"product": {
					"senderPresaveId": sender_id,
					"productId": product_id_value.clone(),
					"medicinalProduct": "Duplicate Product ID"
				}, "activeSubstances": []
			} } }),
			"product_id",
		),
		(
			json!({ "data": { "rows": {
				"product": {
					"senderPresaveId": sender_id,
					"preApprovalIpName": ip_name_value.clone(),
					"medicinalProduct": "Duplicate IP Name"
				}, "activeSubstances": []
			} } }),
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
	assert_eq!(
		details["data"]["rows"]["product"]["id"],
		product_id.to_string()
	);
	assert_eq!(
		details["data"]["rows"]["activeSubstances"][0]["id"],
		substance_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
		json!({ "data": { "rows": {
				"product": { "drugBrandName": "Graph Brand" },
				"activeSubstances": [
					{
						"id": substance_id,
						"sequenceNumber": 2,
						"substanceName": "Substance Updated",
						"substanceStrengthValue": "7.5",
						"substanceStrengthUnit": "mg"
					},
					{
						"sequenceNumber": 3,
						"substanceName": "Substance Created"
					}
				]
		} } }),
	)
	.await?;
	assert_eq!(
		saved["data"]["rows"]["product"]["drugBrandName"],
		"Graph Brand"
	);
	assert_eq!(
		saved["data"]["rows"]["activeSubstances"]
			.as_array()
			.unwrap()
			.len(),
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
		json!({ "data": { "rows": { "product": {
			"mfdsMpid": "KR-MPID",
			"mfdsMpidVersion": "KR-V1"
		} } } }),
	)
	.await?;
	assert_eq!(saved["data"]["rows"]["product"]["mfdsMpid"], "KR-MPID");
	assert!(saved["data"]["rows"]["product"]
		.get("unknown_extra_product_code")
		.is_none());
	assert!(saved["data"]["rows"]["product"]
		.get("unknown_extra_udl_product_code")
		.is_none());
	assert!(saved["data"]["rows"]["product"]
		.get("unknown_extra_foreign_ich_product_code")
		.is_none());
	assert!(saved["data"]["rows"]["product"]
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
	assert!(loaded["data"]["rows"]["product"]
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
		json!({ "data": { "rows": { "product": { "drugBrandName": "Product Noop" } } } }),
	)
	.await?;
	let after_omit = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
	)
	.await?;
	assert_eq!(
		after_omit["data"]["rows"]["activeSubstances"]
			.as_array()
			.unwrap()
			.len(),
		2
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
		json!({ "data": { "rows": { "activeSubstances": [] } } }),
	)
	.await?;
	let after_empty = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
	)
	.await?;
	assert_eq!(
		after_empty["data"]["rows"]["activeSubstances"]
			.as_array()
			.unwrap()
			.len(),
		2
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
		json!({ "data": { "rows": {
			"activeSubstances": [{ "id": substance_delete, "deleted": true }]
		} } }),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
	)
	.await?;
	let active_substances = after_delete["data"]["rows"]["activeSubstances"]
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
		Some(
			json!({ "data": { "rows": { "activeSubstances": [{ "deleted": true }] } } }),
		),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/products/{product_a}/details"),
		Some(json!({
			"data": { "rows": {
				"activeSubstances": [{
					"id": wrong_parent_substance,
					"sequenceNumber": 2,
					"substanceName": "Wrong Parent"
				}]
			} }
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
	let sender_id = details["data"]["rows"]["product"]["senderPresaveId"].clone();
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
