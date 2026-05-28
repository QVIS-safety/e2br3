use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_users, system_user_id,
	Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use lib_core::model::presave::{ProductPresaveBmc, ProductPresaveForCreate};
use lib_core::regulatory::RegulatoryAuthority;
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

fn data_id(value: &Value) -> Result<Uuid> {
	let id = value["data"]["id"].as_str().ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

async fn create_product_presave(
	mm: &lib_core::model::ModelManager,
	org_id: Uuid,
	user_id: Uuid,
) -> Result<Uuid> {
	let ctx = Ctx::new(user_id, org_id, ROLE_SPONSOR_ADMIN_CRO.to_string())?;
	let id = ProductPresaveBmc::create(
		&ctx,
		mm,
		ProductPresaveForCreate {
			authority: RegulatoryAuthority::Fda,
			name: format!("REST Product {}", Uuid::new_v4()),
			comments: None,
			sender_presave_id: None,
			drug_characterization: None,
			medicinal_product: Some("REST Product".into()),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: None,
			drug_generic_name: None,
			manufacturer_name: None,
			product_description: None,
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			drug_authorization_country: None,
			drug_authorization_holder: None,
			holder_applicant_name_notation: None,
			fda_ind_number_occurred: None,
			fda_pre_anda_number_occurred: None,
			mfds_domestic_product_code: None,
			mfds_domestic_ingredient_code: None,
			mfds_udl_product_code: None,
			mfds_udl_ingredient_code: None,
			mfds_udl_manufacturer_code: None,
			mfds_udl_manufacturer_name: None,
			mfds_foreign_ich_product_code: None,
			mfds_foreign_ich_ingredient_code: None,
			mfds_foreign_ich_holder_code: None,
			mfds_foreign_ich_holder_name: None,
			mfds_foreign_e2b_product_code: None,
			mfds_foreign_e2b_ingredient_code: None,
			mfds_foreign_e2b_holder_code: None,
			mfds_foreign_e2b_holder_name: None,
		},
	)
	.await?;
	Ok(id)
}

async fn expect_json_status(
	app: &Router,
	cookie: &str,
	method: Method,
	uri: String,
	body: Option<Value>,
	expected: StatusCode,
) -> Result<Value> {
	let (status, value) = request_json(app, cookie, method, uri, body).await?;
	assert_eq!(status, expected, "{value:?}");
	Ok(value)
}

async fn get_json_ok(app: &Router, cookie: &str, uri: String) -> Result<Value> {
	expect_json_status(app, cookie, Method::GET, uri, None, StatusCode::OK).await
}

async fn post_json_created(
	app: &Router,
	cookie: &str,
	uri: String,
	body: Value,
) -> Result<Value> {
	expect_json_status(
		app,
		cookie,
		Method::POST,
		uri,
		Some(body),
		StatusCode::CREATED,
	)
	.await
}

async fn put_json_ok(
	app: &Router,
	cookie: &str,
	uri: String,
	body: Value,
) -> Result<Value> {
	expect_json_status(app, cookie, Method::PUT, uri, Some(body), StatusCode::OK)
		.await
}

async fn create_sender_presave_via_api(
	app: &Router,
	cookie: &str,
	authority: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/senders".to_string(),
		json!({
			"data": {
				"authority": authority,
				"name": format!("REST Sender Details {}", Uuid::new_v4()),
				"sender_type": "1",
				"organization_name": "REST Sender Details Org",
				"country_code": "US",
				"email": "sender-details@example.com"
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_sender_gateway_via_api(
	app: &Router,
	cookie: &str,
	sender_id: Uuid,
	sequence_number: i32,
	sender_identifier: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/senders/{sender_id}/gateways"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"gateway_authority": "fda",
				"sender_identifier": sender_identifier,
				"routing_identifier": format!("ROUTE-{sender_identifier}"),
				"is_default_for_authority": false
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_sender_responsible_person_via_api(
	app: &Router,
	cookie: &str,
	sender_id: Uuid,
	sequence_number: i32,
	given_name: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/senders/{sender_id}/responsible-persons"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"department": "Safety",
				"person_given_name": given_name,
				"person_family_name": "Kim"
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_receiver_presave_via_api(
	app: &Router,
	cookie: &str,
	authority: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/receivers".to_string(),
		json!({
			"data": {
				"authority": authority,
				"name": format!("REST Receiver Details {}", Uuid::new_v4()),
				"receiver_type": "1",
				"organization_name": "REST Receiver Details Org",
				"receiver_identifier": format!("REC-{}", Uuid::new_v4())
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_receiver_consignee_via_api(
	app: &Router,
	cookie: &str,
	receiver_id: Uuid,
	sequence_number: i32,
	name: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/receivers/{receiver_id}/consignees"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"name": name,
				"email": format!("{}@example.com", name.to_ascii_lowercase())
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_product_presave_via_api(
	app: &Router,
	cookie: &str,
	authority: &str,
) -> Result<Uuid> {
	create_named_product_presave_via_api(
		app,
		cookie,
		authority,
		format!("REST Product Details {}", Uuid::new_v4()),
		"REST Product Details",
	)
	.await
}

async fn create_named_product_presave_via_api(
	app: &Router,
	cookie: &str,
	authority: &str,
	name: String,
	medicinal_product: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/products".to_string(),
		json!({
			"data": {
				"authority": authority,
				"name": name,
				"medicinal_product": medicinal_product
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_product_substance_via_api(
	app: &Router,
	cookie: &str,
	product_id: Uuid,
	sequence_number: i32,
	substance_name: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/products/{product_id}/substances"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"substance_name": substance_name,
				"strength_value": "5",
				"strength_unit": "mg"
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_product_fda_cross_reported_ind_via_api(
	app: &Router,
	cookie: &str,
	product_id: Uuid,
	sequence_number: i32,
	ind_number: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/products/{product_id}/fda-cross-reported-inds"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"ind_number": ind_number
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_product_mfds_regional_item_via_api(
	app: &Router,
	cookie: &str,
	product_id: Uuid,
	sequence_number: i32,
	item_value: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/products/{product_id}/mfds-regional-items"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"item_type": "domestic_product_code",
				"item_value": item_value
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_study_presave_for_product_via_api(
	app: &Router,
	cookie: &str,
	product_id: Uuid,
	authority: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/studies".to_string(),
		json!({
			"data": {
				"authority": authority,
				"name": format!("REST Study Details {}", Uuid::new_v4()),
				"product_presave_id": product_id,
				"study_name": "REST Study Details"
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_study_registration_number_via_api(
	app: &Router,
	cookie: &str,
	study_id: Uuid,
	sequence_number: i32,
	registration_number: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/studies/{study_id}/registration-numbers"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"registration_number": registration_number,
				"country_code": "US"
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_study_fda_cross_reported_ind_via_api(
	app: &Router,
	cookie: &str,
	study_id: Uuid,
	sequence_number: i32,
	ind_number: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/studies/{study_id}/fda-cross-reported-inds"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"ind_number": ind_number
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_narrative_presave_with_authority_via_api(
	app: &Router,
	cookie: &str,
	authority: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/narratives".to_string(),
		json!({
			"data": {
				"authority": authority,
				"name": format!("REST Narrative Details {}", Uuid::new_v4()),
				"case_narrative": "REST narrative details"
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_narrative_sender_diagnosis_with_code_via_api(
	app: &Router,
	cookie: &str,
	narrative_id: Uuid,
	sequence_number: i32,
	code: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/narratives/{narrative_id}/sender-diagnoses"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"diagnosis_meddra_version": "26.1",
				"diagnosis_meddra_code": code
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_narrative_case_summary_with_text_via_api(
	app: &Router,
	cookie: &str,
	narrative_id: Uuid,
	sequence_number: i32,
	summary_text: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/narratives/{narrative_id}/case-summaries"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"summary_type": "sender",
				"language_code": "en",
				"summary_text": summary_text
			}
		}),
	)
	.await?;
	data_id(&value)
}

async fn create_narrative_presave_via_api(
	app: &Router,
	cookie: &str,
) -> Result<Uuid> {
	create_narrative_presave_with_authority_via_api(app, cookie, "ich").await
}

async fn create_narrative_sender_diagnosis_via_api(
	app: &Router,
	cookie: &str,
	narrative_id: Uuid,
	sequence_number: i32,
	code: &str,
) -> Result<Uuid> {
	create_narrative_sender_diagnosis_with_code_via_api(
		app,
		cookie,
		narrative_id,
		sequence_number,
		code,
	)
	.await
}

async fn create_narrative_case_summary_via_api(
	app: &Router,
	cookie: &str,
	narrative_id: Uuid,
	sequence_number: i32,
	summary_text: &str,
) -> Result<Uuid> {
	create_narrative_case_summary_with_text_via_api(
		app,
		cookie,
		narrative_id,
		sequence_number,
		summary_text,
	)
	.await
}

async fn create_template(
	app: &Router,
	cookie: &str,
	entity_type: &str,
	name: &str,
	data: Value,
) -> Result<(Uuid, Value)> {
	let (id, row) =
		create_template_with_authority(app, cookie, entity_type, name, None, data)
			.await?;
	Ok((id, row["data"].clone()))
}

async fn create_template_with_authority(
	app: &Router,
	cookie: &str,
	entity_type: &str,
	name: &str,
	authority: Option<&str>,
	data: Value,
) -> Result<(Uuid, Value)> {
	let mut template = json!({
		"entity_type": entity_type,
		"name": name,
		"description": format!("template for {entity_type}"),
		"data": data
	});
	if let Some(authority) = authority {
		template["authority"] = json!(authority);
	}
	let body = json!({ "data": template });
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
	Ok((Uuid::parse_str(id)?, value["data"].clone()))
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

async fn update_user_scope(
	app: &Router,
	admin_cookie: &str,
	user_id: Uuid,
	body: Value,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		admin_cookie,
		Method::PUT,
		format!("/api/users/{user_id}"),
		Some(json!({ "data": body })),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update user scope failed: status={status} body={value}"
		)
		.into());
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_section_presave_study_rest_contract() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let product_id = create_product_presave(&mm, seed.org_id, seed.admin.id).await?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/studies".to_string(),
		Some(json!({
			"data": {
				"authority": "fda",
				"name": "REST Study Missing Product",
				"study_name": "Missing Product Study"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/studies".to_string(),
		Some(json!({
			"data": {
				"authority": "fda",
				"name": "REST Study",
				"product_presave_id": product_id,
				"study_name": "REST Study Name",
				"study_name_notation": "REST notation",
				"sponsor_study_number": "REST-ST-001",
				"sponsor_study_number_kind": "study_no",
				"fda_ind_number_occurred": "IND-REST"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let study_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		"/api/presaves/studies?authority=fda".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]
			.as_array()
			.ok_or("study list data is not array")?
			.iter()
			.any(|row| row["id"].as_str() == Some(&study_id.to_string())),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		format!("/api/presaves/studies/{study_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["sponsor_study_number_kind"].as_str(),
		Some("study_no")
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/studies/{study_id}"),
		Some(json!({
			"data": {
				"sponsor_study_number_kind": "protocol_no",
				"sponsor_study_number": "REST-PROT-001"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["sponsor_study_number_kind"].as_str(),
		Some("protocol_no")
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/studies/{study_id}/registration-numbers"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"registration_number": "REG-REST",
				"country_code": "US"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let registration_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!(
			"/api/presaves/studies/{study_id}/registration-numbers/{registration_id}"
		),
		Some(json!({ "data": { "deleted": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["deleted"].as_bool(), Some(true));

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/studies/{study_id}/fda-cross-reported-inds"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"ind_number": "IND-CHILD-REST"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let ind_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		format!("/api/presaves/studies/{study_id}/fda-cross-reported-inds"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]
			.as_array()
			.ok_or("study IND list data is not array")?
			.iter()
			.any(|row| row["id"].as_str() == Some(&ind_id.to_string())),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/studies/{study_id}/fda-cross-reported-inds/{ind_id}"),
		Some(json!({ "data": { "deleted": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["deleted"].as_bool(), Some(true));

	let study_delete_uris = [
		format!("/api/presaves/studies/{study_id}/fda-cross-reported-inds/{ind_id}"),
		format!(
			"/api/presaves/studies/{study_id}/registration-numbers/{registration_id}"
		),
		format!("/api/presaves/studies/{study_id}"),
	];
	for uri in study_delete_uris {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::DELETE, uri.clone(), None)
				.await?;
		assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");

		let (status, value) =
			request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert_eq!(value["data"]["deleted"].as_bool(), Some(true));
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_section_presave_narrative_rest_contract() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/narratives".to_string(),
		Some(json!({
			"data": {
				"authority": "ich",
				"name": "REST Narrative Missing Auto Narrative"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/narratives".to_string(),
		Some(json!({
			"data": {
				"authority": "ich",
				"name": "REST Narrative",
				"case_narrative": "REST auto narrative",
				"case_narrative_notation": "REST notation"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let narrative_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		"/api/presaves/narratives?authority=ich".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]
			.as_array()
			.ok_or("narrative list data is not array")?
			.iter()
			.any(|row| row["id"].as_str() == Some(&narrative_id.to_string())),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/narratives/{narrative_id}"),
		Some(json!({
			"data": {
				"case_narrative": "REST auto narrative updated"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["case_narrative"].as_str(),
		Some("REST auto narrative updated")
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/narratives/{narrative_id}/sender-diagnoses"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"diagnosis_meddra_version": "26.1",
				"diagnosis_meddra_code": "10000001"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let diagnosis_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!(
			"/api/presaves/narratives/{narrative_id}/sender-diagnoses/{diagnosis_id}"
		),
		Some(json!({ "data": { "deleted": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["deleted"].as_bool(), Some(true));

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/narratives/{narrative_id}/case-summaries"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"summary_type": "sender",
				"language_code": "en",
				"summary_text": "REST summary"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let summary_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		format!("/api/presaves/narratives/{narrative_id}/case-summaries"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]
			.as_array()
			.ok_or("case summary list data is not array")?
			.iter()
			.any(|row| row["id"].as_str() == Some(&summary_id.to_string())),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!(
			"/api/presaves/narratives/{narrative_id}/case-summaries/{summary_id}"
		),
		Some(json!({ "data": { "deleted": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["deleted"].as_bool(), Some(true));

	let narrative_delete_uris = [
		format!(
			"/api/presaves/narratives/{narrative_id}/case-summaries/{summary_id}"
		),
		format!(
			"/api/presaves/narratives/{narrative_id}/sender-diagnoses/{diagnosis_id}"
		),
		format!("/api/presaves/narratives/{narrative_id}"),
	];
	for uri in narrative_delete_uris {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::DELETE, uri.clone(), None)
				.await?;
		assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");

		let (status, value) =
			request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert_eq!(value["data"]["deleted"].as_bool(), Some(true));
	}

	Ok(())
}

#[serial]
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
				"authority": "ich",
				"name": "REST Sender",
				"sender_type": "1",
				"organization_name": "REST Sender Org",
				"country_code": "US",
				"email": "sender@example.com"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
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
				"authority": "ich",
				"name": "REST Receiver",
				"receiver_type": "1",
				"organization_name": "REST Receiver Org",
				"receiver_identifier": "REST-RECEIVER"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
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
				"authority": "fda",
				"name": "REST Product Canonical",
				"sender_presave_id": sender_id,
				"medicinal_product": "REST Product Canonical",
				"fda_ind_number_occurred": "IND-CANONICAL"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let product_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/products".to_string(),
		Some(json!({
			"data": {
				"authority": "mfds",
				"name": "REST MFDS Product Canonical",
				"sender_presave_id": sender_id,
				"medicinal_product": "REST MFDS Product Canonical"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let mfds_product_id = data_id(&value)?;

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
		format!("/api/presaves/products/{product_id}/fda-cross-reported-inds"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"ind_number": "IND-PRODUCT-REST"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let product_ind_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/products/{mfds_product_id}/mfds-regional-items"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"item_type": "domestic_product_code",
				"item_value": "MFDS-REST"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let regional_item_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/reporters".to_string(),
		Some(json!({
			"data": {
				"authority": "ich",
				"name": "REST Reporter",
				"reporter_given_name": "Grace",
				"reporter_family_name": "Hopper",
				"organization": "REST Reporter Org",
				"country_code": "US"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let reporter_id = data_id(&value)?;

	for (uri, id) in [
		("/api/presaves/senders?authority=ich".to_string(), sender_id),
		(
			"/api/presaves/receivers?authority=ich".to_string(),
			receiver_id,
		),
		(
			"/api/presaves/products?authority=fda".to_string(),
			product_id,
		),
		(
			"/api/presaves/reporters?authority=ich".to_string(),
			reporter_id,
		),
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
		format!(
			"/api/presaves/products/{product_id}/fda-cross-reported-inds/{product_ind_id}"
		),
		format!(
				"/api/presaves/products/{mfds_product_id}/mfds-regional-items/{regional_item_id}"
			),
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
		format!(
			"/api/presaves/products/{product_id}/fda-cross-reported-inds/{product_ind_id}"
		),
		format!(
				"/api/presaves/products/{mfds_product_id}/mfds-regional-items/{regional_item_id}"
			),
		format!("/api/presaves/reporters/{reporter_id}"),
		format!("/api/presaves/products/{mfds_product_id}"),
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

#[serial]
#[tokio::test]
async fn test_sender_presave_details_graph_load_and_save() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "ich").await?;

	let gateway_id =
		create_sender_gateway_via_api(&app, &admin_cookie, sender_id, 1, "SENDER-1")
			.await?;

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], sender_id.to_string());
	assert_eq!(details["data"]["gateways"][0]["id"], gateway_id.to_string());

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({
			"data": {
				"parent": { "comments": "updated by graph" },
				"gateways": [
					{
						"id": gateway_id,
						"sequence_number": 2,
						"gateway_authority": "ema",
						"sender_identifier": "SENDER-2"
					},
					{
						"sequence_number": 3,
						"gateway_authority": "fda",
						"sender_identifier": "SENDER-3"
					}
				],
				"responsible_persons": [
					{
						"sequence_number": 1,
						"department": "Safety",
						"person_given_name": "Ari",
						"person_family_name": "Kim"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(saved["data"]["parent"]["comments"], "updated by graph");
	assert_eq!(saved["data"]["gateways"].as_array().unwrap().len(), 2);
	assert_eq!(
		saved["data"]["responsible_persons"]
			.as_array()
			.unwrap()
			.len(),
		1
	);

	let persisted = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	assert_eq!(
		persisted["data"]["parent"]["comments"].as_str(),
		Some("updated by graph"),
		"{persisted:?}"
	);
	let gateways = persisted["data"]["gateways"].as_array().unwrap();
	assert_eq!(gateways.len(), 2, "{persisted:?}");
	let updated_gateway = gateways
		.iter()
		.find(|row| row["id"].as_str() == Some(&gateway_id.to_string()))
		.ok_or("missing updated gateway")?;
	assert_eq!(
		updated_gateway["sender_identifier"].as_str(),
		Some("SENDER-2")
	);
	assert_eq!(updated_gateway["gateway_authority"].as_str(), Some("ema"));
	assert_eq!(updated_gateway["sequence_number"].as_i64(), Some(2));
	let created_gateway = gateways
		.iter()
		.find(|row| row["sender_identifier"].as_str() == Some("SENDER-3"))
		.ok_or("missing created gateway")?;
	assert_eq!(created_gateway["gateway_authority"].as_str(), Some("fda"));

	let responsible_persons =
		persisted["data"]["responsible_persons"].as_array().unwrap();
	let responsible_person = responsible_persons
		.iter()
		.find(|row| row["person_given_name"].as_str() == Some("Ari"))
		.ok_or("missing responsible person")?;
	assert_eq!(responsible_person["department"].as_str(), Some("Safety"));
	assert_eq!(
		responsible_person["person_family_name"].as_str(),
		Some("Kim")
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_presave_details_rolls_back_parent_on_child_constraint_failure(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "ich").await?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_id}/details"),
		Some(json!({
			"data": {
				"parent": { "comments": "must roll back" },
				"gateways": [{
					"sequence_number": 1,
					"gateway_authority": "ich",
					"sender_identifier": "INVALID-GATEWAY"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let persisted = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	assert_eq!(
		persisted["data"]["parent"]["comments"].as_str(),
		None,
		"{persisted:?}"
	);
	let gateways = persisted["data"]["gateways"].as_array().unwrap();
	assert!(
		!gateways
			.iter()
			.any(|row| row["sender_identifier"].as_str() == Some("INVALID-GATEWAY")),
		"{persisted:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_presave_details_requires_explicit_child_delete() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "ich").await?;
	let gateway_delete_id =
		create_sender_gateway_via_api(&app, &admin_cookie, sender_id, 1, "DELETE")
			.await?;
	let gateway_keep_id =
		create_sender_gateway_via_api(&app, &admin_cookie, sender_id, 2, "KEEP")
			.await?;
	let responsible_id = create_sender_responsible_person_via_api(
		&app,
		&admin_cookie,
		sender_id,
		1,
		"Ari",
	)
	.await?;

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({ "data": { "parent": { "comments": "omit children" } } }),
	)
	.await?;
	let after_omit = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	let gateways = after_omit["data"]["gateways"].as_array().unwrap();
	let responsible_persons = after_omit["data"]["responsible_persons"]
		.as_array()
		.unwrap();
	assert_eq!(gateways.len(), 2);
	assert_eq!(responsible_persons.len(), 1);
	assert!(
		gateways
			.iter()
			.any(|row| row["id"].as_str() == Some(&gateway_delete_id.to_string())),
		"{after_omit:?}"
	);
	assert!(
		gateways
			.iter()
			.any(|row| row["id"].as_str() == Some(&gateway_keep_id.to_string())),
		"{after_omit:?}"
	);
	assert!(
		responsible_persons
			.iter()
			.any(|row| row["id"].as_str() == Some(&responsible_id.to_string())),
		"{after_omit:?}"
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({ "data": { "gateways": [], "responsible_persons": [] } }),
	)
	.await?;
	let after_empty = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	let gateways = after_empty["data"]["gateways"].as_array().unwrap();
	let responsible_persons = after_empty["data"]["responsible_persons"]
		.as_array()
		.unwrap();
	assert_eq!(gateways.len(), 2);
	assert_eq!(responsible_persons.len(), 1);
	assert!(
		gateways
			.iter()
			.any(|row| row["id"].as_str() == Some(&gateway_delete_id.to_string())),
		"{after_empty:?}"
	);
	assert!(
		gateways
			.iter()
			.any(|row| row["id"].as_str() == Some(&gateway_keep_id.to_string())),
		"{after_empty:?}"
	);
	assert!(
		responsible_persons
			.iter()
			.any(|row| row["id"].as_str() == Some(&responsible_id.to_string())),
		"{after_empty:?}"
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({ "data": { "gateways": [{ "id": gateway_delete_id, "_delete": true }] } }),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	let gateways = after_delete["data"]["gateways"].as_array().unwrap();
	let responsible_persons = after_delete["data"]["responsible_persons"]
		.as_array()
		.unwrap();
	assert_eq!(gateways.len(), 1);
	assert_eq!(responsible_persons.len(), 1);
	assert!(
		gateways
			.iter()
			.any(|row| row["id"].as_str() == Some(&gateway_keep_id.to_string())),
		"{after_delete:?}"
	);
	assert!(
		!gateways
			.iter()
			.any(|row| row["id"].as_str() == Some(&gateway_delete_id.to_string())),
		"{after_delete:?}"
	);
	assert!(
		responsible_persons
			.iter()
			.any(|row| row["id"].as_str() == Some(&responsible_id.to_string())),
		"{after_delete:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_presave_details_rejects_invalid_child_operations() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_a = create_sender_presave_via_api(&app, &admin_cookie, "ich").await?;
	let sender_b = create_sender_presave_via_api(&app, &admin_cookie, "ich").await?;
	let gateway_b =
		create_sender_gateway_via_api(&app, &admin_cookie, sender_b, 1, "OTHER")
			.await?;
	let responsible_b = create_sender_responsible_person_via_api(
		&app,
		&admin_cookie,
		sender_b,
		1,
		"Other",
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(json!({ "data": { "gateways": [{ "_delete": true }] } })),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(json!({ "data": { "responsible_persons": [{ "_delete": true }] } })),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(
			json!({ "data": { "gateways": [{ "id": gateway_b, "_delete": true }] } }),
		),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(json!({
			"data": {
				"responsible_persons": [{ "id": responsible_b, "_delete": true }]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(json!({
			"data": {
				"gateways": [{
					"id": gateway_b,
					"sequence_number": 2,
					"gateway_authority": "fda",
					"sender_identifier": "WRONG-PARENT-UPDATE"
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
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(json!({
			"data": {
				"responsible_persons": [{
					"id": responsible_b,
					"sequence_number": 2,
					"department": "Wrong Parent",
					"person_given_name": "Wrong",
					"person_family_name": "Parent"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_receiver_presave_details_graph_load_save_noop_delete_and_invalid(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let receiver_a =
		create_receiver_presave_via_api(&app, &admin_cookie, "ich").await?;
	let receiver_b =
		create_receiver_presave_via_api(&app, &admin_cookie, "ich").await?;
	let consignee_update = create_receiver_consignee_via_api(
		&app,
		&admin_cookie,
		receiver_a,
		1,
		"Update",
	)
	.await?;
	let consignee_delete = create_receiver_consignee_via_api(
		&app,
		&admin_cookie,
		receiver_a,
		2,
		"Delete",
	)
	.await?;
	let wrong_parent_consignee = create_receiver_consignee_via_api(
		&app,
		&admin_cookie,
		receiver_b,
		1,
		"Other",
	)
	.await?;

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/receivers/{receiver_a}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], receiver_a.to_string());
	assert_eq!(details["data"]["consignees"].as_array().unwrap().len(), 2);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/receivers/{receiver_a}/details"),
		json!({
			"data": {
				"parent": { "description": "receiver graph updated" },
				"consignees": [
					{
						"id": consignee_update,
						"sequence_number": 3,
						"name": "Updated Consignee",
						"phone": "555-0100"
					},
					{
						"sequence_number": 4,
						"name": "Created Consignee",
						"email": "created@example.com"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(
		saved["data"]["parent"]["description"],
		"receiver graph updated"
	);
	assert_eq!(saved["data"]["consignees"].as_array().unwrap().len(), 3);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/receivers/{receiver_a}/details"),
		json!({ "data": { "consignees": [] } }),
	)
	.await?;
	let after_noop = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/receivers/{receiver_a}/details"),
	)
	.await?;
	assert_eq!(
		after_noop["data"]["consignees"].as_array().unwrap().len(),
		3
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/receivers/{receiver_a}/details"),
		json!({
			"data": {
				"consignees": [{ "id": consignee_delete, "_delete": true }]
			}
		}),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/receivers/{receiver_a}/details"),
	)
	.await?;
	let consignees = after_delete["data"]["consignees"].as_array().unwrap();
	assert_eq!(consignees.len(), 2, "{after_delete:?}");
	assert!(
		!consignees
			.iter()
			.any(|row| row["id"].as_str() == Some(&consignee_delete.to_string())),
		"{after_delete:?}"
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/receivers/{receiver_a}/details"),
		Some(json!({ "data": { "consignees": [{ "_delete": true }] } })),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/receivers/{receiver_a}/details"),
		Some(json!({
			"data": {
				"consignees": [{
					"id": wrong_parent_consignee,
					"sequence_number": 2,
					"name": "Wrong Parent"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

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
	let substance_id = create_product_substance_via_api(
		&app,
		&admin_cookie,
		product_id,
		1,
		"Substance A",
	)
	.await?;
	let ind_id = create_product_fda_cross_reported_ind_via_api(
		&app,
		&admin_cookie,
		product_id,
		1,
		"IND-OLD",
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
		details["data"]["substances"][0]["id"],
		substance_id.to_string()
	);
	assert_eq!(
		details["data"]["fda_cross_reported_inds"][0]["id"],
		ind_id.to_string()
	);
	assert_eq!(
		details["data"]["mfds_regional_items"]
			.as_array()
			.unwrap()
			.len(),
		0
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
		json!({
			"data": {
				"parent": { "brand_name": "Graph Brand" },
				"substances": [
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
				],
				"fda_cross_reported_inds": [
					{ "id": ind_id, "sequence_number": 2, "ind_number": "IND-UPDATED" },
					{ "sequence_number": 3, "ind_number": "IND-CREATED" }
				],
				"mfds_regional_items": []
			}
		}),
	)
	.await?;
	assert_eq!(saved["data"]["parent"]["brand_name"], "Graph Brand");
	assert_eq!(saved["data"]["substances"].as_array().unwrap().len(), 2);
	assert_eq!(
		saved["data"]["fda_cross_reported_inds"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		saved["data"]["mfds_regional_items"]
			.as_array()
			.unwrap()
			.len(),
		0
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_product_presave_details_mfds_regional_items_load_and_save(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let product_id =
		create_product_presave_via_api(&app, &admin_cookie, "mfds").await?;
	let item_id = create_product_mfds_regional_item_via_api(
		&app,
		&admin_cookie,
		product_id,
		1,
		"MFDS-OLD",
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
		details["data"]["mfds_regional_items"][0]["id"],
		item_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_id}/details"),
		json!({
			"data": {
				"mfds_regional_items": [
					{
						"id": item_id,
						"sequence_number": 2,
						"item_type": "domestic_product_code",
						"item_value": "MFDS-UPDATED"
					},
					{
						"sequence_number": 3,
						"item_type": "domestic_ingredient_code",
						"item_value": "MFDS-CREATED"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(
		saved["data"]["mfds_regional_items"]
			.as_array()
			.unwrap()
			.len(),
		2
	);

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
	let substance_delete = create_product_substance_via_api(
		&app,
		&admin_cookie,
		product_a,
		1,
		"Delete Substance",
	)
	.await?;
	let substance_keep = create_product_substance_via_api(
		&app,
		&admin_cookie,
		product_a,
		2,
		"Keep Substance",
	)
	.await?;
	let wrong_parent_substance = create_product_substance_via_api(
		&app,
		&admin_cookie,
		product_b,
		1,
		"Other Product Substance",
	)
	.await?;
	let ind_keep = create_product_fda_cross_reported_ind_via_api(
		&app,
		&admin_cookie,
		product_a,
		1,
		"IND-KEEP",
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
		after_omit["data"]["substances"].as_array().unwrap().len(),
		2
	);
	assert_eq!(
		after_omit["data"]["fda_cross_reported_inds"]
			.as_array()
			.unwrap()
			.len(),
		1
	);
	assert_eq!(
		after_omit["data"]["mfds_regional_items"]
			.as_array()
			.unwrap()
			.len(),
		0
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
		json!({
			"data": {
				"substances": [],
				"fda_cross_reported_inds": [],
				"mfds_regional_items": []
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
		after_empty["data"]["substances"].as_array().unwrap().len(),
		2
	);
	assert_eq!(
		after_empty["data"]["fda_cross_reported_inds"]
			.as_array()
			.unwrap()
			.len(),
		1
	);
	assert_eq!(
		after_empty["data"]["mfds_regional_items"]
			.as_array()
			.unwrap()
			.len(),
		0
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/products/{product_a}/details"),
		json!({
			"data": {
				"substances": [{ "id": substance_delete, "_delete": true }]
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
	let substances = after_delete["data"]["substances"].as_array().unwrap();
	assert!(
		!substances
			.iter()
			.any(|row| row["id"].as_str() == Some(&substance_delete.to_string())),
		"{after_delete:?}"
	);
	assert!(
		substances
			.iter()
			.any(|row| row["id"].as_str() == Some(&substance_keep.to_string())),
		"{after_delete:?}"
	);
	assert!(
		after_delete["data"]["fda_cross_reported_inds"]
			.as_array()
			.unwrap()
			.iter()
			.any(|row| row["id"].as_str() == Some(&ind_keep.to_string())),
		"{after_delete:?}"
	);
	assert_eq!(
		after_delete["data"]["mfds_regional_items"]
			.as_array()
			.unwrap()
			.len(),
		0
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/products/{product_a}/details"),
		Some(json!({ "data": { "substances": [{ "_delete": true }] } })),
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
				"substances": [{
					"id": wrong_parent_substance,
					"sequence_number": 2,
					"substance_name": "Wrong Parent"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_product_presave_details_rejects_fda_inds_for_non_fda_products(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let mfds_product =
		create_product_presave_via_api(&app, &admin_cookie, "mfds").await?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/products/{mfds_product}/details"),
		Some(json!({
			"data": {
				"fda_cross_reported_inds": [{
					"sequence_number": 1,
					"ind_number": "IND-NON-FDA-CREATE"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/products/{mfds_product}/fda-cross-reported-inds"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"ind_number": "IND-DIRECT-NON-FDA-CREATE"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_product_presave_details_rejects_mfds_regional_items_for_non_mfds_products(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let fda_product =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/products/{fda_product}/details"),
		Some(json!({
			"data": {
				"mfds_regional_items": [{
					"sequence_number": 1,
					"item_type": "domestic_product_code",
					"item_value": "MFDS-NON-MFDS-CREATE"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/products/{fda_product}/mfds-regional-items"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"item_type": "domestic_product_code",
				"item_value": "MFDS-DIRECT-NON-MFDS-CREATE"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

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
	let ind_id = create_study_fda_cross_reported_ind_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		"IND-OLD",
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
		details["data"]["fda_cross_reported_inds"][0]["id"],
		ind_id.to_string()
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
				"fda_cross_reported_inds": [
					{ "id": ind_id, "sequence_number": 2, "ind_number": "IND-UPDATED" },
					{ "sequence_number": 3, "ind_number": "IND-CREATED" }
				]
			}
		}),
	)
	.await?;
	assert_eq!(saved["data"]["parent"]["study_name"], "Study Graph Updated");
	assert_eq!(saved["data"]["registrations"].as_array().unwrap().len(), 2);
	assert_eq!(
		saved["data"]["fda_cross_reported_inds"]
			.as_array()
			.unwrap()
			.len(),
		2
	);

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
async fn test_narrative_presave_details_graph_load_save_and_delete() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let narrative_id =
		create_narrative_presave_with_authority_via_api(&app, &admin_cookie, "ich")
			.await?;
	let diagnosis_id = create_narrative_sender_diagnosis_with_code_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"10000001",
	)
	.await?;
	let summary_id = create_narrative_case_summary_with_text_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"Summary old",
	)
	.await?;

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], narrative_id.to_string());
	assert_eq!(
		details["data"]["sender_diagnoses"][0]["id"],
		diagnosis_id.to_string()
	);
	assert_eq!(
		details["data"]["case_summaries"][0]["id"],
		summary_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({
			"data": {
				"parent": { "case_narrative": "Narrative graph updated" },
				"sender_diagnoses": [
					{
						"id": diagnosis_id,
						"sequence_number": 2,
						"diagnosis_meddra_version": "26.1",
						"diagnosis_meddra_code": "10000002"
					},
					{
						"sequence_number": 3,
						"diagnosis_meddra_version": "26.1",
						"diagnosis_meddra_code": "10000003"
					}
				],
				"case_summaries": [
					{
						"id": summary_id,
						"sequence_number": 2,
						"summary_type": "sender",
						"language_code": "en",
						"summary_text": "Summary updated"
					},
					{
						"sequence_number": 3,
						"summary_type": "reporter",
						"language_code": "en",
						"summary_text": "Summary created"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(
		saved["data"]["parent"]["case_narrative"],
		"Narrative graph updated"
	);
	assert_eq!(
		saved["data"]["sender_diagnoses"].as_array().unwrap().len(),
		2
	);
	assert_eq!(saved["data"]["case_summaries"].as_array().unwrap().len(), 2);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({
			"data": {
				"case_summaries": [{ "id": summary_id, "_delete": true }]
			}
		}),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	let deleted_summary = after_delete["data"]["case_summaries"]
		.as_array()
		.unwrap()
		.iter()
		.find(|row| row["id"].as_str() == Some(&summary_id.to_string()))
		.ok_or("missing deleted summary")?
		.clone();
	assert_eq!(deleted_summary["deleted"].as_bool(), Some(true));

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
	let ind_id = create_study_fda_cross_reported_ind_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		"IND-1",
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
		details["data"]["fda_cross_reported_inds"][0]["id"],
		ind_id.to_string()
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
				"fda_cross_reported_inds": [
					{
						"id": ind_id,
						"sequence_number": 2,
						"ind_number": "IND-2"
					},
					{
						"sequence_number": 3,
						"ind_number": "IND-3"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(
		saved["data"]["parent"]["comments"],
		"updated by study graph"
	);
	assert_eq!(saved["data"]["registrations"].as_array().unwrap().len(), 2);
	assert_eq!(
		saved["data"]["fda_cross_reported_inds"]
			.as_array()
			.unwrap()
			.len(),
		2
	);

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

	let inds = persisted["data"]["fda_cross_reported_inds"]
		.as_array()
		.unwrap();
	assert!(
		inds.iter()
			.any(|row| row["ind_number"].as_str() == Some("IND-2")),
		"{persisted:?}"
	);
	assert!(
		inds.iter()
			.any(|row| row["ind_number"].as_str() == Some("IND-3")),
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
	let ind_id = create_study_fda_cross_reported_ind_via_api(
		&app,
		&admin_cookie,
		study_id,
		1,
		"KEEP-IND",
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
	assert_eq!(
		after_omit["data"]["fda_cross_reported_inds"]
			.as_array()
			.unwrap()
			.len(),
		1
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({ "data": { "registrations": [], "fda_cross_reported_inds": [] } }),
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
	assert_eq!(
		after_empty["data"]["fda_cross_reported_inds"]
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
		after_delete["data"]["fda_cross_reported_inds"]
			.as_array()
			.unwrap()
			.iter()
			.any(|row| row["id"].as_str() == Some(&ind_id.to_string())),
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
	let ind_b = create_study_fda_cross_reported_ind_via_api(
		&app,
		&admin_cookie,
		study_b,
		1,
		"OTHER-IND",
	)
	.await?;

	for body in [
		json!({ "data": { "registrations": [{ "_delete": true }] } }),
		json!({ "data": { "fda_cross_reported_inds": [{ "_delete": true }] } }),
		json!({ "data": { "registrations": [{ "id": registration_b, "_delete": true }] } }),
		json!({ "data": { "fda_cross_reported_inds": [{ "id": ind_b, "_delete": true }] } }),
		json!({ "data": { "registrations": [{ "id": registration_b, "sequence_number": 2, "registration_number": "WRONG" }] } }),
		json!({ "data": { "fda_cross_reported_inds": [{ "id": ind_b, "sequence_number": 2, "ind_number": "WRONG" }] } }),
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

#[serial]
#[tokio::test]
async fn test_narrative_presave_details_graph_load_and_save() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let narrative_id = create_narrative_presave_via_api(&app, &admin_cookie).await?;
	let diagnosis_id = create_narrative_sender_diagnosis_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"10000001",
	)
	.await?;
	let summary_id = create_narrative_case_summary_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"Summary 1",
	)
	.await?;

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], narrative_id.to_string());
	assert_eq!(
		details["data"]["sender_diagnoses"][0]["id"],
		diagnosis_id.to_string()
	);
	assert_eq!(
		details["data"]["case_summaries"][0]["id"],
		summary_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({
			"data": {
				"parent": { "case_narrative": "updated by narrative graph" },
				"sender_diagnoses": [
					{
						"id": diagnosis_id,
						"sequence_number": 2,
						"diagnosis_meddra_version": "27.0",
						"diagnosis_meddra_code": "10000002"
					},
					{
						"sequence_number": 3,
						"diagnosis_meddra_version": "27.0",
						"diagnosis_meddra_code": "10000003"
					}
				],
				"case_summaries": [
					{
						"id": summary_id,
						"sequence_number": 2,
						"summary_type": "company",
						"language_code": "en",
						"summary_text": "Summary 2"
					},
					{
						"sequence_number": 3,
						"summary_type": "sender",
						"language_code": "ko",
						"summary_text": "Summary 3"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(
		saved["data"]["parent"]["case_narrative"],
		"updated by narrative graph"
	);
	assert_eq!(
		saved["data"]["sender_diagnoses"].as_array().unwrap().len(),
		2
	);
	assert_eq!(saved["data"]["case_summaries"].as_array().unwrap().len(), 2);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_presave_details_requires_explicit_child_delete() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let narrative_id = create_narrative_presave_via_api(&app, &admin_cookie).await?;
	let diagnosis_delete_id = create_narrative_sender_diagnosis_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"DELETE",
	)
	.await?;
	let diagnosis_keep_id = create_narrative_sender_diagnosis_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		2,
		"KEEP",
	)
	.await?;
	let summary_id = create_narrative_case_summary_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"Keep summary",
	)
	.await?;

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({ "data": { "parent": { "comments": "omit children" } } }),
	)
	.await?;
	let after_omit = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	assert_eq!(
		after_omit["data"]["sender_diagnoses"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		after_omit["data"]["case_summaries"]
			.as_array()
			.unwrap()
			.len(),
		1
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({ "data": { "sender_diagnoses": [], "case_summaries": [] } }),
	)
	.await?;
	let after_empty = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	assert_eq!(
		after_empty["data"]["sender_diagnoses"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		after_empty["data"]["case_summaries"]
			.as_array()
			.unwrap()
			.len(),
		1
	);

	let after_delete = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({
			"data": {
				"sender_diagnoses": [
					{ "id": diagnosis_delete_id, "_delete": true }
				]
			}
		}),
	)
	.await?;
	let diagnoses = after_delete["data"]["sender_diagnoses"].as_array().unwrap();
	let deleted_diagnosis = diagnoses
		.iter()
		.find(|row| row["id"].as_str() == Some(&diagnosis_delete_id.to_string()))
		.ok_or("missing deleted diagnosis")?;
	assert_eq!(deleted_diagnosis["deleted"].as_bool(), Some(true));
	let kept_diagnosis = diagnoses
		.iter()
		.find(|row| row["id"].as_str() == Some(&diagnosis_keep_id.to_string()))
		.ok_or("missing kept diagnosis")?;
	assert_eq!(kept_diagnosis["deleted"].as_bool(), Some(false));
	assert!(
		after_delete["data"]["case_summaries"]
			.as_array()
			.unwrap()
			.iter()
			.any(|row| row["id"].as_str() == Some(&summary_id.to_string())),
		"{after_delete:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_presave_details_rejects_invalid_child_operations(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let narrative_a = create_narrative_presave_via_api(&app, &admin_cookie).await?;
	let narrative_b = create_narrative_presave_via_api(&app, &admin_cookie).await?;
	let diagnosis_b = create_narrative_sender_diagnosis_via_api(
		&app,
		&admin_cookie,
		narrative_b,
		1,
		"OTHER",
	)
	.await?;
	let summary_b = create_narrative_case_summary_via_api(
		&app,
		&admin_cookie,
		narrative_b,
		1,
		"Other summary",
	)
	.await?;

	for body in [
		json!({ "data": { "sender_diagnoses": [{ "_delete": true }] } }),
		json!({ "data": { "case_summaries": [{ "_delete": true }] } }),
		json!({ "data": { "sender_diagnoses": [{ "id": diagnosis_b, "_delete": true }] } }),
		json!({ "data": { "case_summaries": [{ "id": summary_b, "_delete": true }] } }),
		json!({ "data": { "sender_diagnoses": [{ "id": diagnosis_b, "sequence_number": 2, "diagnosis_meddra_code": "WRONG" }] } }),
		json!({ "data": { "case_summaries": [{ "id": summary_b, "sequence_number": 2, "summary_text": "Wrong parent" }] } }),
		json!({ "data": { "parent": { "case_narrative": " " } } }),
	] {
		let (status, value) = request_json(
			&app,
			&admin_cookie,
			Method::PUT,
			format!("/api/presaves/narratives/{narrative_a}/details"),
			Some(body),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	}

	Ok(())
}

async fn create_info_reader(
	app: &Router,
	mm: &lib_core::model::ModelManager,
	admin_cookie: &str,
	org_id: Uuid,
) -> Result<(Uuid, String)> {
	let role_name = format!("presave_reader_{}", Uuid::new_v4().simple());
	let (status, value) = request_json(
		app,
		admin_cookie,
		Method::POST,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": role_name,
				"description": "Presave scope reader",
				"privileges": [
					{
						"menu_key": "info",
						"can_read": true,
						"can_edit": false,
						"can_review": false,
						"can_lock": false
					}
				]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let role_id = value["id"].as_str().ok_or("missing role id")?.to_string();
	let user =
		insert_user(mm, org_id, &role_id, system_user_id(), Some("readerpwd"))
			.await?;
	let token = generate_web_token(&user.email, user.token_salt)?;
	Ok((user.id, cookie_header(&token.to_string())))
}

async fn create_info_editor(
	app: &Router,
	mm: &lib_core::model::ModelManager,
	admin_cookie: &str,
	org_id: Uuid,
) -> Result<(Uuid, String)> {
	let role_name = format!("presave_editor_{}", Uuid::new_v4().simple());
	let (status, value) = request_json(
		app,
		admin_cookie,
		Method::POST,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": role_name,
				"description": "Presave scope editor",
				"privileges": [
					{
						"menu_key": "info",
						"can_read": true,
						"can_edit": true,
						"can_review": false,
						"can_lock": false
					}
				]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let role_id = value["id"].as_str().ok_or("missing role id")?.to_string();
	let user =
		insert_user(mm, org_id, &role_id, system_user_id(), Some("editorpwd"))
			.await?;
	let token = generate_web_token(&user.email, user.token_salt)?;
	Ok((user.id, cookie_header(&token.to_string())))
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
async fn test_presave_product_list_follows_assigned_product_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (viewer_id, viewer_cookie) =
		create_info_reader(&app, &mm, &admin_cookie, seed.org_id).await?;

	let (visible_id, _) = create_template(
		&app,
		&admin_cookie,
		"product",
		"visible-product-template",
		json!({
			"medicinalProduct": "VISIBLE-PRODUCT",
			"drugGenericName": "Visible Generic"
		}),
	)
	.await?;
	let (hidden_id, _) = create_template(
		&app,
		&admin_cookie,
		"product",
		"hidden-product-template",
		json!({
			"medicinalProduct": "HIDDEN-PRODUCT",
			"drugGenericName": "Hidden Generic"
		}),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		viewer_id,
		json!({ "access_product_ids": ["VISIBLE-PRODUCT"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/presave-templates?entityType=product".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let rows = value["data"]
		.as_array()
		.ok_or("presave template list data is not an array")?;
	assert!(
		rows.iter()
			.any(|row| row["id"].as_str() == Some(&visible_id.to_string())),
		"{value:?}"
	);
	assert!(
		!rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&hidden_id.to_string())),
		"{value:?}"
	);
	let (status, value) = get_template(&app, &viewer_cookie, visible_id).await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let (status, value) = get_template(&app, &viewer_cookie, hidden_id).await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_update_delete_respect_assigned_product_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (editor_id, editor_cookie) =
		create_info_editor(&app, &mm, &admin_cookie, seed.org_id).await?;

	let (visible_id, _) = create_template(
		&app,
		&admin_cookie,
		"product",
		"visible-product-template-for-edit",
		json!({
			"medicinalProduct": "VISIBLE-PRODUCT-EDIT",
			"drugGenericName": "Visible Edit Generic"
		}),
	)
	.await?;
	let (hidden_id, _) = create_template(
		&app,
		&admin_cookie,
		"product",
		"hidden-product-template-for-edit",
		json!({
			"medicinalProduct": "HIDDEN-PRODUCT-EDIT",
			"drugGenericName": "Hidden Edit Generic"
		}),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		editor_id,
		json!({ "access_product_ids": ["VISIBLE-PRODUCT-EDIT"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::POST,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "product",
				"name": "out-of-scope product create",
				"description": "Should be rejected for scoped editor",
				"data": {
					"medicinalProduct": "HIDDEN-PRODUCT-CREATED",
					"drugGenericName": "Hidden Created Generic"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::POST,
		"/api/presave-templates".to_string(),
		Some(json!({
			"data": {
				"entity_type": "product",
				"name": "out-of-scope product create with decoy",
				"description": "Nested decoy productId must not grant scope",
				"data": {
					"medicinalProduct": "HIDDEN-PRODUCT-DECOY",
					"drugGenericName": "Hidden Decoy Generic",
					"metadata": {
						"productId": "VISIBLE-PRODUCT-EDIT"
					}
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PATCH,
		format!("/api/presave-templates/{hidden_id}"),
		Some(json!({
			"data": {
				"name": "hidden product edited out of scope",
				"data": {
					"medicinalProduct": "HIDDEN-PRODUCT-EDITED"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PATCH,
		format!("/api/presave-templates/{visible_id}"),
		Some(json!({
			"data": {
				"name": "visible product moved out of scope with decoy",
				"data": {
					"medicinalProduct": "HIDDEN-PRODUCT-DECOY-MOVED",
					"drugGenericName": "Hidden Decoy Moved Generic",
					"metadata": {
						"productId": "VISIBLE-PRODUCT-EDIT"
					}
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PATCH,
		format!("/api/presave-templates/{visible_id}"),
		Some(json!({
			"data": {
				"name": "visible product moved out of scope",
				"data": {
					"medicinalProduct": "HIDDEN-PRODUCT-MOVED",
					"drugGenericName": "Hidden Moved Generic"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::DELETE,
		format!("/api/presave-templates/{hidden_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PATCH,
		format!("/api/presave-templates/{visible_id}"),
		Some(json!({
			"data": {
				"name": "visible product edited in scope",
				"data": {
					"medicinalProduct": "VISIBLE-PRODUCT-EDIT",
					"drugGenericName": "Visible Edit Generic Updated"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) = get_template(&app, &admin_cookie, hidden_id).await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["name"].as_str(),
		Some("hidden-product-template-for-edit"),
		"{value:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_canonical_product_presaves_respect_assigned_product_scope(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (editor_id, editor_cookie) =
		create_info_editor(&app, &mm, &admin_cookie, seed.org_id).await?;

	let visible_id = create_named_product_presave_via_api(
		&app,
		&admin_cookie,
		"fda",
		"visible canonical product".to_string(),
		"VISIBLE-CANONICAL-PRODUCT",
	)
	.await?;
	let hidden_id = create_named_product_presave_via_api(
		&app,
		&admin_cookie,
		"fda",
		"hidden canonical product".to_string(),
		"HIDDEN-CANONICAL-PRODUCT",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		editor_id,
		json!({ "access_product_ids": ["VISIBLE-CANONICAL-PRODUCT"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::GET,
		"/api/presaves/products?authority=fda".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let rows = value["data"]
		.as_array()
		.ok_or("canonical product list data is not an array")?;
	assert!(
		rows.iter()
			.any(|row| row["id"].as_str() == Some(&visible_id.to_string())),
		"{value:?}"
	);
	assert!(
		!rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&hidden_id.to_string())),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::POST,
		"/api/presaves/products".to_string(),
		Some(json!({
			"data": {
				"authority": "fda",
				"name": "out-of-scope canonical product create",
				"medicinal_product": "HIDDEN-CANONICAL-CREATED"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	for uri in [
		format!("/api/presaves/products/{hidden_id}"),
		format!("/api/presaves/products/{hidden_id}/details"),
	] {
		let (status, value) =
			request_json(&app, &editor_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	}

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PATCH,
		format!("/api/presaves/products/{hidden_id}"),
		Some(json!({
			"data": {
				"name": "hidden canonical product edited"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::DELETE,
		format!("/api/presaves/products/{hidden_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PUT,
		format!("/api/presaves/products/{visible_id}/details"),
		Some(json!({
			"data": {
				"parent": {
					"name": "visible canonical product details edit"
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_canonical_product_parent_soft_delete_requires_delete_permission(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (_editor_id, editor_cookie) =
		create_info_editor(&app, &mm, &admin_cookie, seed.org_id).await?;

	let patch_id =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PATCH,
		format!("/api/presaves/products/{patch_id}"),
		Some(json!({
			"data": {
				"deleted": true
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let details_id =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PUT,
		format!("/api/presaves/products/{details_id}/details"),
		Some(json!({
			"data": {
				"parent": {
					"deleted": true
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_sender_list_follows_assigned_sender_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (viewer_id, viewer_cookie) =
		create_info_reader(&app, &mm, &admin_cookie, seed.org_id).await?;

	let (visible_id, _) = create_template_with_authority(
		&app,
		&admin_cookie,
		"sender",
		"visible-sender-template",
		Some("fda"),
		json!({
			"senderIdentifier": "SENDER-VISIBLE",
			"senderOrganization": "Visible Sender"
		}),
	)
	.await?;
	let (hidden_id, _) = create_template_with_authority(
		&app,
		&admin_cookie,
		"sender",
		"hidden-sender-template",
		Some("fda"),
		json!({
			"senderIdentifier": "SENDER-HIDDEN",
			"senderOrganization": "Hidden Sender"
		}),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		viewer_id,
		json!({ "access_sender_ids": ["SENDER-VISIBLE"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/presave-templates?entityType=sender&authority=fda".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let rows = value["data"]
		.as_array()
		.ok_or("presave template list data is not an array")?;
	assert!(
		rows.iter()
			.any(|row| row["id"].as_str() == Some(&visible_id.to_string())),
		"{value:?}"
	);
	assert!(
		!rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&hidden_id.to_string())),
		"{value:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_study_list_follows_assigned_study_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (viewer_id, viewer_cookie) =
		create_info_reader(&app, &mm, &admin_cookie, seed.org_id).await?;

	let (visible_id, _) = create_template(
		&app,
		&admin_cookie,
		"study",
		"visible-study-template",
		json!({
			"studyName": "Visible Study",
			"sponsorStudyNumber": "STUDY-VISIBLE"
		}),
	)
	.await?;
	let (hidden_id, _) = create_template(
		&app,
		&admin_cookie,
		"study",
		"hidden-study-template",
		json!({
			"studyName": "Hidden Study",
			"sponsorStudyNumber": "STUDY-HIDDEN"
		}),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		viewer_id,
		json!({ "access_study_ids": ["STUDY-VISIBLE"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/presave-templates?entityType=study".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let rows = value["data"]
		.as_array()
		.ok_or("presave template list data is not an array")?;
	assert!(
		rows.iter()
			.any(|row| row["id"].as_str() == Some(&visible_id.to_string())),
		"{value:?}"
	);
	assert!(
		!rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&hidden_id.to_string())),
		"{value:?}"
	);
	let (status, value) = get_template(&app, &viewer_cookie, visible_id).await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let (status, value) = get_template(&app, &viewer_cookie, hidden_id).await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	Ok(())
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
async fn test_presave_templates_filter_by_authority_and_include_global() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let (global_id, global) = create_template_with_authority(
		&app,
		&admin_cookie,
		"sender",
		"global-sender",
		None,
		json!({ "senderIdentifier": "GLOBAL-SENDER" }),
	)
	.await?;
	assert_eq!(global["authority"], Value::Null);

	let (fda_id, fda) = create_template_with_authority(
		&app,
		&admin_cookie,
		"sender",
		"fda-sender",
		Some("fda"),
		json!({ "senderIdentifier": "FDA-SENDER" }),
	)
	.await?;
	assert_eq!(fda["authority"], json!("fda"));

	let (mfds_id, mfds) = create_template_with_authority(
		&app,
		&admin_cookie,
		"sender",
		"mfds-sender",
		Some("mfds"),
		json!({ "senderIdentifier": "MFDS-SENDER" }),
	)
	.await?;
	assert_eq!(mfds["authority"], json!("mfds"));

	let (status, list) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		"/api/presave-templates?entityType=sender&authority=fda".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{list:?}");
	let ids: Vec<Uuid> = list["data"]
		.as_array()
		.ok_or("presave template list data is not an array")?
		.iter()
		.map(|row| {
			let id = row["id"].as_str().ok_or("missing id")?;
			Ok(Uuid::parse_str(id)?)
		})
		.collect::<Result<Vec<_>>>()?;
	assert!(ids.contains(&fda_id), "{list:?}");
	assert!(ids.contains(&global_id), "{list:?}");
	assert!(!ids.contains(&mfds_id), "{list:?}");
	assert_eq!(
		ids.first(),
		Some(&fda_id),
		"authority-specific row should sort before global rows"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_sender_default_is_org_level_singleton() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (first_id, _) = create_template(
		&app,
		&cookie,
		"sender",
		"default-sender-one",
		json!({
			"senderType": "1",
			"senderIdentifier": "DEFAULT-SENDER-ONE",
			"senderOrganization": "Default Sender One",
			"senderDefault": true
		}),
	)
	.await?;
	let (second_id, _) = create_template(
		&app,
		&cookie,
		"sender",
		"default-sender-two",
		json!({
			"senderType": "1",
			"senderIdentifier": "DEFAULT-SENDER-TWO",
			"senderOrganization": "Default Sender Two",
			"senderDefault": true
		}),
	)
	.await?;

	let (status, first) = get_template(&app, &cookie, first_id).await?;
	assert_eq!(status, StatusCode::OK, "{first:?}");
	assert_eq!(first["data"]["data"]["senderDefault"], false);

	let (status, second) = get_template(&app, &cookie, second_id).await?;
	assert_eq!(status, StatusCode::OK, "{second:?}");
	assert_eq!(second["data"]["data"]["senderDefault"], true);

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presave-templates/{first_id}"),
		Some(json!({
			"data": {
				"data": {
					"senderType": "1",
					"senderIdentifier": "DEFAULT-SENDER-ONE",
					"senderOrganization": "Default Sender One",
					"senderDefault": true
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, first) = get_template(&app, &cookie, first_id).await?;
	assert_eq!(status, StatusCode::OK, "{first:?}");
	assert_eq!(first["data"]["data"]["senderDefault"], true);

	let (status, second) = get_template(&app, &cookie, second_id).await?;
	assert_eq!(status, StatusCode::OK, "{second:?}");
	assert_eq!(second["data"]["data"]["senderDefault"], false);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_sender_default_is_authority_scoped() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let (fda_one_id, _) = create_template_with_authority(
		&app,
		&admin_cookie,
		"sender",
		"fda-default-one",
		Some("fda"),
		json!({ "senderIdentifier": "FDA-ONE", "senderDefault": true }),
	)
	.await?;
	let (mfds_id, _) = create_template_with_authority(
		&app,
		&admin_cookie,
		"sender",
		"mfds-default",
		Some("mfds"),
		json!({ "senderIdentifier": "MFDS-ONE", "senderDefault": true }),
	)
	.await?;
	let (fda_two_id, _) = create_template_with_authority(
		&app,
		&admin_cookie,
		"sender",
		"fda-default-two",
		Some("fda"),
		json!({ "senderIdentifier": "FDA-TWO", "senderDefault": true }),
	)
	.await?;

	let (_, fda_one) = get_template(&app, &admin_cookie, fda_one_id).await?;
	let (_, fda_two) = get_template(&app, &admin_cookie, fda_two_id).await?;
	let (_, mfds) = get_template(&app, &admin_cookie, mfds_id).await?;
	assert_eq!(fda_one["data"]["data"]["senderDefault"], false);
	assert_eq!(fda_two["data"]["data"]["senderDefault"], true);
	assert_eq!(mfds["data"]["data"]["senderDefault"], true);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_non_sender_sender_default_flag_does_not_clear_default_sender(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (first_id, _) = create_template(
		&app,
		&cookie,
		"sender",
		"default-sender-one",
		json!({
			"senderType": "1",
			"senderIdentifier": "DEFAULT-SENDER-ONE",
			"senderOrganization": "Default Sender One",
			"senderDefault": true
		}),
	)
	.await?;
	let (second_id, _) = create_template(
		&app,
		&cookie,
		"sender",
		"default-sender-two",
		json!({
			"senderType": "1",
			"senderIdentifier": "DEFAULT-SENDER-TWO",
			"senderOrganization": "Default Sender Two",
			"senderDefault": true
		}),
	)
	.await?;
	let (product_id, _) = create_template(
		&app,
		&cookie,
		"product",
		"product-with-legacy-default-key",
		json!({
			"drugCharacterization": "1",
			"medicinalProduct": "Product With Legacy Key"
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::PATCH,
		format!("/api/presave-templates/{product_id}"),
		Some(json!({
			"data": {
				"data": {
					"drugCharacterization": "1",
					"medicinalProduct": "Product With Legacy Key",
					"senderDefault": true
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, first) = get_template(&app, &cookie, first_id).await?;
	assert_eq!(status, StatusCode::OK, "{first:?}");
	assert_eq!(first["data"]["data"]["senderDefault"], false);

	let (status, second) = get_template(&app, &cookie, second_id).await?;
	assert_eq!(status, StatusCode::OK, "{second:?}");
	assert_eq!(second["data"]["data"]["senderDefault"], true);

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

	let (template_id, _) = create_template_with_authority(
		&app,
		&cookie,
		"sender",
		"sender-contract-update",
		Some("fda"),
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
	assert!(
		rows.iter().any(|row| {
			row["new_values"]["authority"] == json!("fda")
				|| row["old_values"]["authority"] == json!("fda")
		}),
		"{audit:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_presave_audit_respects_assigned_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (viewer_id, viewer_cookie) =
		create_info_reader(&app, &mm, &admin_cookie, seed.org_id).await?;

	let (hidden_id, _) = create_template(
		&app,
		&admin_cookie,
		"product",
		"hidden-product-template",
		json!({
			"medicinalProduct": "HIDDEN-PRODUCT",
			"drugGenericName": "Hidden Generic"
		}),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		viewer_id,
		json!({ "access_product_ids": ["VISIBLE-PRODUCT"] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		format!("/api/presave-templates/{hidden_id}/audit"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

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
