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

async fn create_case(app: &Router, cookie: &str, org_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": org_id,
			"safety_report_id": format!("PS-{}", Uuid::new_v4()),
			"status": "draft",
			"validation_profile": "fda"
		}
	});
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		"/api/cases".to_string(),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create case failed: status={status} body={value}").into()
		);
	}
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing case data.id")?;
	Ok(Uuid::parse_str(id)?)
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
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing template data.id")?;
	Ok((Uuid::parse_str(id)?, value["data"]["data"].clone()))
}

async fn get_template(
	app: &Router,
	cookie: &str,
	template_id: Uuid,
) -> Result<Value> {
	let (status, value) = request_json(
		app,
		cookie,
		Method::GET,
		format!("/api/presave-templates/{template_id}"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get template failed: status={status} template_id={template_id} body={value}"
		)
		.into());
	}
	Ok(value)
}

fn add_case_id(mut data: Value, case_id: Uuid) -> Result<Value> {
	let obj = data
		.as_object_mut()
		.ok_or("template data must be an object for import")?;
	obj.insert("case_id".to_string(), json!(case_id));
	Ok(data)
}

async fn import_template_on_case_create(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	entity_type: &str,
	data: Value,
) -> Result<()> {
	let (uri, payload) = match entity_type {
		"sender" => (
			format!("/api/cases/{case_id}/safety-report/senders"),
			add_case_id(data, case_id)?,
		),
		"receiver" => (
			format!("/api/cases/{case_id}/receiver"),
			add_case_id(data, case_id)?,
		),
		"product" => (
			format!("/api/cases/{case_id}/drugs"),
			add_case_id(data, case_id)?,
		),
		"reporter" => (
			format!("/api/cases/{case_id}/safety-report/primary-sources"),
			add_case_id(data, case_id)?,
		),
		"study" => (
			format!("/api/cases/{case_id}/safety-report/studies"),
			add_case_id(data, case_id)?,
		),
		"narrative" => (
			format!("/api/cases/{case_id}/narrative"),
			add_case_id(data, case_id)?,
		),
		_ => {
			return Err(
				format!("unsupported template entity_type: {entity_type}").into()
			)
		}
	};

	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		uri,
		Some(json!({ "data": payload })),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"import template {entity_type} failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_templates_info_section_supports_all_six_entity_types(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let entities = [
		(
			"sender",
			json!({"sender_type": "1", "organization_name": "PS Sender Org"}),
		),
		(
			"receiver",
			json!({"receiver_type": "1", "organization_name": "PS Receiver Org"}),
		),
		(
			"product",
			json!({"sequence_number": 1, "drug_characterization": "1", "medicinal_product": "PS Product"}),
		),
		(
			"reporter",
			json!({"sequence_number": 1, "qualification": "1", "email": "reporter@example.com"}),
		),
		(
			"study",
			json!({"study_name": "PS Study", "sponsor_study_number": "PS-STUDY-001"}),
		),
		("narrative", json!({"case_narrative": "PS narrative text"})),
	];

	for (entity_type, data) in entities {
		let (template_id, _) = create_template(
			&app,
			&cookie,
			entity_type,
			&format!("{entity_type}-template"),
			data.clone(),
		)
		.await?;

		let (status, value) = request_json(
			&app,
			&cookie,
			Method::GET,
			format!("/api/presave-templates/{template_id}"),
			None,
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert_eq!(
			value["data"]["entity_type"].as_str(),
			Some(entity_type),
			"{value:?}"
		);

		let (status, list) = request_json(
			&app,
			&cookie,
			Method::GET,
			format!("/api/presave-templates?entityType={entity_type}"),
			None,
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{list:?}");
		let arr = list["data"]
			.as_array()
			.ok_or("presave template list data is not an array")?;
		assert!(
			arr.iter()
				.any(|row| row["id"].as_str() == Some(&template_id.to_string())),
			"template {template_id} not found in list for {entity_type}: {list:?}"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_info_presave_templates_import_cleanly_on_new_case_creation(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let templates = [
		(
			"sender",
			json!({"sender_type": "1", "organization_name": "PS Sender Org"}),
		),
		(
			"receiver",
			json!({"receiver_type": "1", "organization_name": "PS Receiver Org"}),
		),
		(
			"product",
			json!({"sequence_number": 1, "drug_characterization": "1", "medicinal_product": "PS Product"}),
		),
		(
			"reporter",
			json!({"sequence_number": 1, "qualification": "1", "email": "reporter@example.com"}),
		),
		(
			"study",
			json!({"study_name": "PS Study", "sponsor_study_number": "PS-STUDY-001"}),
		),
		("narrative", json!({"case_narrative": "PS narrative text"})),
	];

	let mut imported = Vec::new();
	for (entity_type, data) in templates {
		let (_id, payload) = create_template(
			&app,
			&cookie,
			entity_type,
			&format!("import-{entity_type}"),
			data,
		)
		.await?;
		imported.push((entity_type.to_string(), payload));
	}

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	for (entity_type, payload) in imported {
		import_template_on_case_create(
			&app,
			&cookie,
			case_id,
			entity_type.as_str(),
			payload,
		)
		.await?;
	}

	let (status, sender) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/senders"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{sender:?}");
	assert!(sender["data"]
		.as_array()
		.map(|v| !v.is_empty())
		.unwrap_or(false));

	let (status, receiver) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/receiver"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{receiver:?}");
	assert_eq!(
		receiver["data"]["organization_name"].as_str(),
		Some("PS Receiver Org")
	);

	let (status, drugs) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/drugs"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{drugs:?}");
	assert!(drugs["data"]
		.as_array()
		.map(|v| !v.is_empty())
		.unwrap_or(false));

	let (status, reporters) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{reporters:?}");
	assert!(reporters["data"]
		.as_array()
		.map(|v| !v.is_empty())
		.unwrap_or(false));

	let (status, studies) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/safety-report/studies"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{studies:?}");
	assert!(studies["data"]
		.as_array()
		.map(|v| !v.is_empty())
		.unwrap_or(false));

	let (status, narrative) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{narrative:?}");
	assert_eq!(
		narrative["data"]["case_narrative"].as_str(),
		Some("PS narrative text")
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_new_presave_templates_are_saved_correctly_with_audit() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let templates = [
		(
			"sender",
			json!({"sender_type": "1", "organization_name": "Saved Sender Org"}),
		),
		(
			"receiver",
			json!({"receiver_type": "1", "organization_name": "Saved Receiver Org"}),
		),
		(
			"product",
			json!({"sequence_number": 1, "drug_characterization": "1", "medicinal_product": "Saved Product"}),
		),
		(
			"reporter",
			json!({"sequence_number": 1, "qualification": "1", "email": "saved.reporter@example.com"}),
		),
		(
			"study",
			json!({"study_name": "Saved Study", "sponsor_study_number": "SAVED-STUDY-001"}),
		),
		(
			"narrative",
			json!({"case_narrative": "Saved narrative text"}),
		),
	];

	for (entity_type, data) in templates {
		let name = format!("saved-{entity_type}");
		let (template_id, created_data) =
			create_template(&app, &cookie, entity_type, &name, data.clone()).await?;
		assert_eq!(created_data, data, "create returned mismatched data");

		let saved = get_template(&app, &cookie, template_id).await?;
		assert_eq!(
			saved["data"]["entity_type"].as_str(),
			Some(entity_type),
			"{saved:?}"
		);
		assert_eq!(
			saved["data"]["name"].as_str(),
			Some(name.as_str()),
			"{saved:?}"
		);
		assert_eq!(saved["data"]["data"], data, "{saved:?}");

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
		assert!(
			rows.iter().any(|row| {
				row["action"].as_str() == Some("CREATE")
					&& row["template_id"].as_str() == Some(&template_id.to_string())
			}),
			"missing CREATE audit row for template {template_id}: {audit:?}"
		);
	}

	Ok(())
}
