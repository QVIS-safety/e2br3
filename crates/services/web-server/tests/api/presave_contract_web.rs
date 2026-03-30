use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

fn parse_json_or_raw(body: &[u8]) -> Value {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return json!({});
	}
	serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({ "raw": raw }))
}

async fn request_json(
	app: &Router,
	cookie: &str,
	method: Method,
	uri: String,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let req =
		builder.body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, parse_json_or_raw(&bytes)))
}

async fn create_template(
	app: &Router,
	cookie: &str,
	entity_type: &str,
	name: &str,
	data: Value,
) -> Result<(Uuid, Value)> {
	let body = json!({
		"data": {
			"entity_type": entity_type,
			"name": name,
			"description": format!("template for {entity_type}"),
			"data": data
		}
	});
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		"/api/presave-templates".to_string(),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create presave template {entity_type} failed: status={status} body={value}"
		)
		.into());
	}
	let id = value["data"]["id"]
		.as_str()
		.ok_or("missing template data.id")?;
	Ok((Uuid::parse_str(id)?, value["data"]["data"].clone()))
}

async fn get_template(
	app: &Router,
	cookie: &str,
	template_id: Uuid,
) -> Result<(StatusCode, Value)> {
	request_json(
		app,
		cookie,
		Method::GET,
		format!("/api/presave-templates/{template_id}"),
		None,
	)
	.await
}

fn sample_presave_payload(entity_type: &str) -> Value {
	match entity_type {
		"sender" => json!({
			"senderType": "1",
			"senderOrganization": "PS Sender Org",
			"senderDepartment": "PV",
			"senderPersonTitle": "Dr",
			"senderPersonGivenName": "Alice",
			"senderPersonFamilyName": "Kim",
			"senderStreetAddress": "1 Safety Way",
			"senderCity": "Seoul",
			"senderCountryCode": "KR",
			"senderEmail": "sender@example.com"
		}),
		"receiver" => json!({
			"receiverType": "2",
			"organizationName": "PS Receiver Org",
			"department": "Submission Ops",
			"contactEmail": "receiver@example.com",
			"routingRules": [{
				"authority": "fda",
				"reportType": "1",
				"batchReceiverIdentifier": "ZZFDA",
				"messageReceiverIdentifier": "CDER"
			}]
		}),
		"product" => json!({
			"drugCharacterization": "1",
			"medicinalProduct": "PS Product",
			"drugGenericName": "Generic PS Product",
			"drugBrandName": "Brand PS Product",
			"drugAuthorizationNumber": "AUTH-PS-001",
			"activeSubstances": [{
				"substanceName": "Substance PS",
				"substanceTermId": "TERM-PS-001",
				"substanceTermIdVersion": "TERM-V1",
				"substanceStrengthValue": 5.0,
				"substanceStrengthUnit": "mg"
			}]
		}),
		"reporter" => json!({
			"reporterGivenName": "Reporter",
			"reporterOrganization": "Reporter Org",
			"reporterFamilyName": "Kim",
			"reporterCountry": "KR",
			"reporterEmail": "reporter@example.com",
			"qualification": "1"
		}),
		"study" => json!({
			"studyName": "PS Study",
			"sponsorStudyNumber": "PS-STUDY-001",
			"studyTypeReaction": "2",
			"studyRegistrationNumber": "REG-PS-001",
			"studyRegistrationCountry": "US"
		}),
		"narrative" => json!({
			"caseNarrative": "PS narrative text",
			"reporterComments": "Reporter PS comments",
			"senderComments": "Sender PS comments",
			"caseSummary": "PS summary"
		}),
		other => panic!("unexpected entity_type {other}"),
	}
}

#[serial]
#[tokio::test]
async fn test_presave_contract_supports_all_six_entity_types() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for entity_type in [
		"sender",
		"receiver",
		"product",
		"reporter",
		"study",
		"narrative",
	] {
		let data = sample_presave_payload(entity_type);
		let (template_id, created_data) = create_template(
			&app,
			&cookie,
			entity_type,
			&format!("{entity_type}-template"),
			data.clone(),
		)
		.await?;
		assert_eq!(created_data, data);

		let (status, value) = get_template(&app, &cookie, template_id).await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert_eq!(value["data"]["entity_type"].as_str(), Some(entity_type));
		assert_eq!(value["data"]["data"], data, "{value:?}");

		let (status, list) = request_json(
			&app,
			&cookie,
			Method::GET,
			format!("/api/presave-templates?entityType={entity_type}"),
			None,
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{list:?}");
		let rows = list["data"]
			.as_array()
			.ok_or("presave template list data is not an array")?;
		assert!(rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&template_id.to_string())));
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_contract_update_delete_and_audit() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie,
		"sender",
		"sender-contract-update",
		json!({
			"senderType": "1",
			"senderOrganization": "Original Sender Org"
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presave-templates/{template_id}"),
		Some(json!({
			"data": {
				"name": "sender-contract-update-2",
				"description": "updated description",
				"data": {
					"senderType": "1",
					"senderOrganization": "Updated Sender Org",
					"senderDepartment": "Safety"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, updated) = get_template(&app, &cookie, template_id).await?;
	assert_eq!(status, StatusCode::OK, "{updated:?}");
	assert_eq!(
		updated["data"]["name"].as_str(),
		Some("sender-contract-update-2")
	);
	assert_eq!(
		updated["data"]["data"]["senderOrganization"].as_str(),
		Some("Updated Sender Org")
	);
	assert_eq!(
		updated["data"]["data"]["senderDepartment"].as_str(),
		Some("Safety")
	);

	let (status, _) = request_json(
		&app,
		&cookie,
		Method::DELETE,
		format!("/api/presave-templates/{template_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::NO_CONTENT);

	let (status, value) = get_template(&app, &cookie, template_id).await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{value:?}");

	let (status, audit) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/presave-templates/{template_id}/audit"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{audit:?}");
	let rows = audit["data"]
		.as_array()
		.ok_or("template audit data is not an array")?;
	let actions = rows
		.iter()
		.filter_map(|row| row["action"].as_str())
		.collect::<Vec<_>>();
	assert!(actions.contains(&"CREATE"), "{audit:?}");
	assert!(actions.contains(&"UPDATE"), "{audit:?}");
	assert!(actions.contains(&"DELETE"), "{audit:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_contract_rejects_invalid_entity_type() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::POST,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "bogus",
				"name": "invalid-entity",
				"data": {}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{value:?}");
	assert!(
		value.to_string().contains("invalid presave entity type")
			|| value.to_string().contains("unknown variant")
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_contract_enforces_org_isolation() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed_a = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let seed_b = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token_a = generate_web_token(&seed_a.admin.email, seed_a.admin.token_salt)?;
	let token_b = generate_web_token(&seed_b.admin.email, seed_b.admin.token_salt)?;
	let cookie_a = cookie_header(&token_a.to_string());
	let cookie_b = cookie_header(&token_b.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie_a,
		"study",
		"isolated-study",
		json!({"studyName": "Org A Study"}),
	)
	.await?;

	let (status, value) = get_template(&app, &cookie_b, template_id).await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{value:?}");

	let (status, list) = request_json(
		&app,
		&cookie_b,
		Method::GET,
		"/api/presave-templates?entityType=study".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{list:?}");
	let rows = list["data"]
		.as_array()
		.ok_or("presave template list data is not an array")?;
	assert!(
		!rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&template_id.to_string())),
		"{list:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_contract_write_requires_admin() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::POST,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "sender",
				"name": "viewer-should-not-create",
				"data": {
					"senderType": "1",
					"senderOrganization": "Nope"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	Ok(())
}
