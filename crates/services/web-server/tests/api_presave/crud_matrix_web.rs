use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
async fn test_section_presave_sender_receiver_product_reporter_rest_contract(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/senders".to_string(),
		Some(json!({ "data": { "rows": {
			"sender": { "senderType": "1", "organizationName": "REST Sender Org", "countryCode": "US", "email": "sender@example.com" },
			"gateways": [], "responsiblePersons": []
		} } })),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(
		value["data"]["rows"]["sender"].get("name").is_none(),
		"{value:?}"
	);
	assert!(
		value["data"]["rows"]["sender"].get("comments").is_none(),
		"{value:?}"
	);
	let sender_id = data_rows_id(&value, "sender")?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/senders/{sender_id}/gateways"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"gateway_authority": "fda",
				"sender_identifier": "REST-SENDER"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let gateway_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/senders/{sender_id}/responsible-persons"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"person_given_name": "Ada",
				"person_family_name": "Lovelace"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let responsible_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/receivers".to_string(),
		Some(json!({ "data": { "rows": {
			"receiver": { "receiverType": "Regulatory Authority", "organizationName": "REST Receiver Org" },
			"consignees": [], "routes": []
		} } })),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(
		value["data"]["rows"]["receiver"].get("name").is_none(),
		"{value:?}"
	);
	assert!(
		value["data"]["rows"]["receiver"].get("comments").is_none(),
		"{value:?}"
	);
	let receiver_id = data_rows_id(&value, "receiver")?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/receivers/{receiver_id}/consignees"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"name": "REST Consignee",
				"email": "consignee@example.com"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let consignee_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/products".to_string(),
		Some(json!({ "data": { "rows": {
			"product": { "senderPresaveId": sender_id, "productId": "REST-PRODUCT-CANONICAL", "medicinalProduct": "REST Product Canonical" },
			"activeSubstances": []
		} } })),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(
		value["data"]["rows"]["product"].get("name").is_none(),
		"{value:?}"
	);
	assert!(
		value["data"]["rows"]["product"].get("comments").is_none(),
		"{value:?}"
	);
	let product_id = data_rows_id(&value, "product")?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/products/{product_id}/active-substances"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"substance_name": "REST Substance",
				"strength_value": "10.5",
				"strength_unit": "mg"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let substance_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/reporters".to_string(),
		Some(json!({ "data": { "rows": { "reporter": {
				"reporterGivenName": "Grace",
				"reporterFamilyName": "Hopper",
				"organization": "REST Reporter Org",
				"countryCode": "US",
				"qualification": "1"
			} } } })),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(
		value["data"]["rows"]["reporter"].get("name").is_none(),
		"{value:?}"
	);
	assert!(
		value["data"]["rows"]["reporter"].get("comments").is_none(),
		"{value:?}"
	);
	let reporter_id = data_rows_id(&value, "reporter")?;

	for (uri, id) in [
		("/api/presaves/senders".to_string(), sender_id),
		("/api/presaves/receivers".to_string(), receiver_id),
		("/api/presaves/products".to_string(), product_id),
		("/api/presaves/reporters".to_string(), reporter_id),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert!(
			value["data"]
				.as_array()
				.ok_or("presave list data is not array")?
				.iter()
				.any(|row| {
					row["id"].as_str() == Some(&id.to_string())
						|| row["rows"]
							.as_object()
							.and_then(|rows| {
								rows.values().find_map(|value| value["id"].as_str())
							})
							.is_some_and(|value| value == id.to_string())
				}),
			"{value:?}"
		);
	}

	for uri in [
		format!("/api/presaves/senders/{sender_id}/gateways/{gateway_id}"),
		format!(
			"/api/presaves/senders/{sender_id}/responsible-persons/{responsible_id}"
		),
		format!("/api/presaves/receivers/{receiver_id}/consignees/{consignee_id}"),
		format!(
			"/api/presaves/products/{product_id}/active-substances/{substance_id}"
		),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
	}

	for (uri, body, field, expected) in [
		(
			format!("/api/presaves/senders/{sender_id}"),
			json!({ "data": { "organizationName": "REST Sender Org Updated" } }),
			"organizationName",
			"REST Sender Org Updated",
		),
		(
			format!("/api/presaves/receivers/{receiver_id}"),
			json!({ "data": { "description": "REST receiver updated" } }),
			"description",
			"REST receiver updated",
		),
		(
			format!("/api/presaves/products/{product_id}"),
			json!({ "data": { "drugBrandName": "REST Brand Updated" } }),
			"drugBrandName",
			"REST Brand Updated",
		),
		(
			format!("/api/presaves/reporters/{reporter_id}"),
			json!({ "data": { "rows": { "reporter": { "reporterGivenName": "Grace Updated" } } } }),
			"reporterGivenName",
			"Grace Updated",
		),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::PATCH, uri, Some(body))
				.await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		let response_row = value["data"]["rows"]
			.as_object()
			.and_then(|rows| rows.values().next())
			.unwrap_or(&value["data"]);
		assert_eq!(response_row[field].as_str(), Some(expected));
	}

	for uri in [
		format!("/api/presaves/senders/{sender_id}/gateways/{gateway_id}"),
		format!(
			"/api/presaves/senders/{sender_id}/responsible-persons/{responsible_id}"
		),
		format!("/api/presaves/receivers/{receiver_id}/consignees/{consignee_id}"),
		format!(
			"/api/presaves/products/{product_id}/active-substances/{substance_id}"
		),
		format!("/api/presaves/reporters/{reporter_id}"),
		format!("/api/presaves/products/{product_id}"),
		format!("/api/presaves/receivers/{receiver_id}"),
		format!("/api/presaves/senders/{sender_id}"),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::DELETE, uri.clone(), None)
				.await?;
		assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");
	}

	Ok(())
}

#[tokio::test]
#[serial]
async fn presave_optional_text_blank_null_and_omitted_contract() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let created = post_json_created(
		&app,
		&cookie,
		"/api/presaves/senders".to_string(),
		json!({ "data": { "rows": {
			"sender": {
				"senderType": "1",
				"organizationName": "Blank Contract Sender",
				"city": "   ",
				"email": "seed@example.test"
			},
			"gateways": [],
			"responsiblePersons": []
		} } }),
	)
	.await?;
	let sender_id = data_rows_id(&created, "sender")?;
	assert_eq!(
		created["data"]["rows"]["sender"].get("city"),
		Some(&json!(null))
	);

	let (status, omitted) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presaves/senders/{sender_id}"),
		Some(json!({ "data": { "fax": "123" } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{omitted:?}");
	assert_eq!(omitted["data"]["email"], "seed@example.test");

	for value in [json!(null), json!(""), json!("   ")] {
		let (status, seeded) = request_json(
			&app,
			&cookie,
			Method::PATCH,
			format!("/api/presaves/senders/{sender_id}"),
			Some(json!({ "data": { "email": "seed@example.test" } })),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{seeded:?}");
		let (status, cleared) = request_json(
			&app,
			&cookie,
			Method::PATCH,
			format!("/api/presaves/senders/{sender_id}"),
			Some(json!({ "data": { "email": value } })),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{cleared:?}");
		assert_eq!(cleared["data"].get("email"), Some(&json!(null)));
	}

	let gateway = post_json_created(
		&app,
		&cookie,
		format!("/api/presaves/senders/{sender_id}/gateways"),
		json!({ "data": {
			"sequence_number": 1,
			"gateway_authority": "fda",
			"sender_identifier": "seed"
		} }),
	)
	.await?;
	let gateway_id = data_id(&gateway)?;
	let (status, cleared) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presaves/senders/{sender_id}/gateways/{gateway_id}"),
		Some(json!({ "data": { "sender_identifier": null } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{cleared:?}");
	assert_eq!(cleared["data"].get("sender_identifier"), Some(&json!(null)));

	let receiver = post_json_created(
		&app,
		&cookie,
		"/api/presaves/receivers".to_string(),
		json!({ "data": { "rows": { "receiver": {
			"receiverType": "Regulatory Authority",
			"organizationName": "Blank Contract Receiver",
			"description": "seed"
		}, "consignees": [], "routes": [] } } }),
	)
	.await?;
	let receiver_id = data_rows_id(&receiver, "receiver")?;
	let (status, receiver) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presaves/receivers/{receiver_id}"),
		Some(json!({ "data": { "description": null } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{receiver:?}");
	assert_eq!(receiver["data"].get("description"), Some(&json!(null)));

	let product = post_json_created(
		&app,
		&cookie,
		"/api/presaves/products".to_string(),
		json!({ "data": { "rows": { "product": {
			"senderPresaveId": sender_id,
			"productId": "BLANK-CONTRACT-PRODUCT",
			"productDescription": "seed"
		}, "activeSubstances": [] } } }),
	)
	.await?;
	let product_id = data_rows_id(&product, "product")?;
	let (status, product) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presaves/products/{product_id}"),
		Some(json!({ "data": { "productDescription": "   " } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{product:?}");
	assert_eq!(
		product["data"].get("product_description"),
		Some(&json!(null))
	);

	let reporter = post_json_created(
		&app,
		&cookie,
		"/api/presaves/reporters".to_string(),
		json!({ "data": { "rows": { "reporter": {
			"reporterGivenName": "Blank Contract Reporter",
			"reporterFamilyName": "seed",
			"organization": "Blank Contract Reporter Org",
			"qualification": "1"
		} } } }),
	)
	.await?;
	let reporter_id = data_rows_id(&reporter, "reporter")?;
	let (status, reporter) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presaves/reporters/{reporter_id}"),
		Some(json!({ "data": { "rows": { "reporter": {
			"reporterFamilyName": ""
		} } } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{reporter:?}");
	assert_eq!(
		reporter["data"]["rows"]["reporter"].get("reporterFamilyName"),
		Some(&json!(null))
	);

	let study = post_json_created(
		&app, &cookie, "/api/presaves/studies".to_string(),
		json!({ "data": { "rows": { "study": {
			"productPresaveId": product_id,
			"studyName": "Blank Contract Study",
			"studyNameNotation": "seed",
			"sponsorStudyNumber": "BLANK-CONTRACT-STUDY",
			"studyTypeReaction": "1"
		}, "products": [], "reporters": [], "registrationNumbers": [], "fdaCrossReportedInds": [] } } }),
	).await?;
	let study_id = data_rows_id(&study, "study")?;
	let (status, study) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presaves/studies/{study_id}"),
		Some(json!({ "data": { "studyNameNotation": null } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{study:?}");
	assert_eq!(study["data"].get("studyNameNotation"), Some(&json!(null)));

	let narrative = post_json_created(
		&app,
		&cookie,
		"/api/presaves/narratives".to_string(),
		json!({ "data": { "rows": { "narrative": {
			"caseNarrative": "Blank contract narrative",
			"templateTitle": "seed"
		} } } }),
	)
	.await?;
	let narrative_id = data_rows_id(&narrative, "narrative")?;
	let (status, narrative) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presaves/narratives/{narrative_id}"),
		Some(json!({ "data": { "rows": { "narrative": {
			"templateTitle": "   "
		} } } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{narrative:?}");
	assert_eq!(
		narrative["data"]["rows"]["narrative"].get("templateTitle"),
		Some(&json!(null))
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn presave_parent_delete_is_repeatable_and_patch_restores_the_same_row(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let sender_id = create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let product_sender_id =
		create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let product_id = create_named_product_presave_for_sender_via_api(
		&app,
		&cookie,
		product_sender_id,
		format!("Lifecycle Product {}", Uuid::new_v4()),
		"Lifecycle Product",
	)
	.await?;
	let receiver_id = create_receiver_presave_via_api(&app, &cookie, "fda").await?;
	let reporter_id = create_named_reporter_presave_via_api(
		&app,
		&cookie,
		format!("Lifecycle Reporter {}", Uuid::new_v4()),
		"Lifecycle Reporter Org",
	)
	.await?;
	let study_product_id =
		create_product_presave(&mm, seed.org_id, seed.admin.id).await?;
	let study_id = create_study_presave_for_product_via_api(
		&app,
		&cookie,
		study_product_id,
		"fda",
	)
	.await?;
	let narrative = post_json_created(
		&app,
		&cookie,
		"/api/presaves/narratives".to_string(),
		json!({"data": {"rows": {"narrative": {
			"caseNarrative": format!("Lifecycle narrative {}", Uuid::new_v4())
		}}}}),
	)
	.await?;
	let narrative_id = data_rows_id(&narrative, "narrative")?;

	for (path, item_pointer, list_path, list_id_pointer, id, restore_body) in [
		(
			format!("/api/presaves/senders/{sender_id}"),
			"/data",
			"/api/presaves/senders".to_string(),
			"/id",
			sender_id,
			json!({"data": {"deleted": false}}),
		),
		(
			format!("/api/presaves/products/{product_id}"),
			"/data",
			"/api/presaves/products".to_string(),
			"/id",
			product_id,
			json!({"data": {"deleted": false}}),
		),
		(
			format!("/api/presaves/receivers/{receiver_id}"),
			"/data",
			"/api/presaves/receivers".to_string(),
			"/id",
			receiver_id,
			json!({"data": {"deleted": false}}),
		),
		(
			format!("/api/presaves/reporters/{reporter_id}"),
			"/data/rows/reporter",
			"/api/presaves/reporters".to_string(),
			"/rows/reporter/id",
			reporter_id,
			json!({"data": {"rows": {"reporter": {"deleted": false}}}}),
		),
		(
			format!("/api/presaves/studies/{study_id}"),
			"/data",
			"/api/presaves/studies".to_string(),
			"/id",
			study_id,
			json!({"data": {"deleted": false}}),
		),
		(
			format!("/api/presaves/narratives/{narrative_id}"),
			"/data/rows/narrative",
			"/api/presaves/narratives".to_string(),
			"/rows/narrative/id",
			narrative_id,
			json!({"data": {"rows": {"narrative": {"deleted": false}}}}),
		),
	] {
		for _ in 0..2 {
			let (status, value) =
				request_json(&app, &cookie, Method::DELETE, path.clone(), None)
					.await?;
			assert_eq!(status, StatusCode::NO_CONTENT, "{path}: {value:?}");
		}

		let deleted = get_json_ok(&app, &cookie, path.clone()).await?;
		let deleted_row = deleted
			.pointer(item_pointer)
			.ok_or("missing deleted parent row")?;
		assert_eq!(deleted_row["id"], id.to_string(), "{path}: {deleted:?}");
		assert_eq!(deleted_row["deleted"], true, "{path}: {deleted:?}");

		for _ in 0..2 {
			let (status, restored) = request_json(
				&app,
				&cookie,
				Method::PATCH,
				path.clone(),
				Some(restore_body.clone()),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{path}: {restored:?}");
			let restored_row = restored
				.pointer(item_pointer)
				.ok_or("missing restored parent row")?;
			assert_eq!(restored_row["id"], id.to_string(), "{path}: {restored:?}");
			assert_eq!(restored_row["deleted"], false, "{path}: {restored:?}");

			let reloaded = get_json_ok(&app, &cookie, path.clone()).await?;
			let reloaded_row = reloaded
				.pointer(item_pointer)
				.ok_or("missing reloaded parent row")?;
			assert_eq!(reloaded_row["id"], id.to_string(), "{path}: {reloaded:?}");
			assert_eq!(reloaded_row["deleted"], false, "{path}: {reloaded:?}");

			let listed = get_json_ok(&app, &cookie, list_path.clone()).await?;
			let rows = listed["data"].as_array().ok_or("missing presave list")?;
			let expected_id = id.to_string();
			let matching_count = rows
				.iter()
				.filter(|row| {
					row.pointer(list_id_pointer).and_then(Value::as_str)
						== Some(expected_id.as_str())
				})
				.count();
			assert_eq!(matching_count, 1, "{list_path}: {listed:?}");
		}
	}

	Ok(())
}
