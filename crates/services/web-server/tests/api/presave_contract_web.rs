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
