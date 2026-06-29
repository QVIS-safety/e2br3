use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;

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
		Some(json!({
			"data": {
				"comments": "legacy sender metadata should be ignored",
				"sender_type": "1",
				"organization_name": "REST Sender Org",
				"country_code": "US",
				"email": "sender@example.com"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(
		value["data"]["name"].as_str(),
		Some("REST Sender Org / sender@example.com")
	);
	assert!(value["data"]["comments"].is_null(), "{value:?}");
	let sender_id = data_id(&value)?;

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
		Some(json!({
			"data": {
				"comments": "legacy receiver metadata should be ignored",
				"receiver_type": "Regulatory Authority",
				"organization_name": "REST Receiver Org",
				"receiver_identifier": "REST-RECEIVER"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(
		value["data"]["name"].as_str(),
		Some("REST Receiver Org / REST-RECEIVER / Regulatory Authority")
	);
	assert!(value["data"]["comments"].is_null(), "{value:?}");
	let receiver_id = data_id(&value)?;

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
		Some(json!({
		"data": {
				"comments": "legacy product metadata should be ignored",
				"sender_presave_id": sender_id,
				"product_id": "REST-PRODUCT-CANONICAL",
				"medicinal_product": "REST Product Canonical"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(
		value["data"]["name"].as_str(),
		Some("REST-PRODUCT-CANONICAL / REST Product Canonical")
	);
	assert!(value["data"]["comments"].is_null(), "{value:?}");
	let product_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/products/{product_id}/substances"),
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
		Some(json!({
			"data": {
				"comments": "legacy reporter metadata should be ignored",
				"reporter_given_name": "Grace",
				"reporter_family_name": "Hopper",
				"organization": "REST Reporter Org",
				"country_code": "US",
				"qualification": "1"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(
		value["data"]["name"].as_str(),
		Some("REST Reporter Org / Grace Hopper")
	);
	assert!(value["data"]["comments"].is_null(), "{value:?}");
	let reporter_id = data_id(&value)?;

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
				.any(|row| row["id"].as_str() == Some(&id.to_string())),
			"{value:?}"
		);
	}

	for uri in [
		format!("/api/presaves/senders/{sender_id}/gateways/{gateway_id}"),
		format!(
			"/api/presaves/senders/{sender_id}/responsible-persons/{responsible_id}"
		),
		format!("/api/presaves/receivers/{receiver_id}/consignees/{consignee_id}"),
		format!("/api/presaves/products/{product_id}/substances/{substance_id}"),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
	}

	for (uri, body, field, expected) in [
		(
			format!("/api/presaves/senders/{sender_id}"),
			json!({ "data": { "organization_name": "REST Sender Org Updated" } }),
			"organization_name",
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
			json!({ "data": { "brand_name": "REST Brand Updated" } }),
			"brand_name",
			"REST Brand Updated",
		),
		(
			format!("/api/presaves/reporters/{reporter_id}"),
			json!({ "data": { "reporter_given_name": "Grace Updated" } }),
			"reporter_given_name",
			"Grace Updated",
		),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::PATCH, uri, Some(body))
				.await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert_eq!(value["data"][field].as_str(), Some(expected));
	}

	for uri in [
		format!("/api/presaves/senders/{sender_id}/gateways/{gateway_id}"),
		format!(
			"/api/presaves/senders/{sender_id}/responsible-persons/{responsible_id}"
		),
		format!("/api/presaves/receivers/{receiver_id}/consignees/{consignee_id}"),
		format!("/api/presaves/products/{product_id}/substances/{substance_id}"),
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
