use crate::common::{cookie_header, insert_user, system_user_id, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use lib_core::model::presave::{
	ProductPresaveBmc, ProductPresaveForCreate, SenderPresaveBmc,
	SenderPresaveForCreate,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

pub(super) fn parse_json_or_raw(body: &[u8]) -> Value {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return json!({});
	}
	serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({ "raw": raw }))
}

pub(super) async fn request_json(
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

pub(super) fn data_id(value: &Value) -> Result<Uuid> {
	let id = value["data"]["id"].as_str().ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

pub(super) async fn create_product_presave(
	mm: &lib_core::model::ModelManager,
	org_id: Uuid,
	user_id: Uuid,
) -> Result<Uuid> {
	let ctx = Ctx::new(user_id, org_id, ROLE_SPONSOR_ADMIN_CRO.to_string())?;
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		mm,
		SenderPresaveForCreate {
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!(
				"REST Product Sender Org {}",
				Uuid::new_v4()
			)),
			organization_name_notation: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;
	let id = ProductPresaveBmc::create(
		&ctx,
		mm,
		ProductPresaveForCreate {
			sender_presave_id: Some(sender_id),
			product_id: Some(format!("REST-PRODUCT-{}", Uuid::new_v4())),
			medicinal_product: Some("REST Product".into()),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: None,
			original_manufacturer: None,
			product_description: None,
			mpid: None,
			mpid_version: None,
			mfds_mpid: None,
			mfds_mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			drug_authorization_country: None,
			drug_authorization_holder: None,
			holder_applicant_name_notation: None,
		},
	)
	.await?;
	Ok(id)
}

pub(super) async fn expect_json_status(
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

pub(super) async fn get_json_ok(
	app: &Router,
	cookie: &str,
	uri: String,
) -> Result<Value> {
	expect_json_status(app, cookie, Method::GET, uri, None, StatusCode::OK).await
}

pub(super) async fn post_json_created(
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

pub(super) async fn put_json_ok(
	app: &Router,
	cookie: &str,
	uri: String,
	body: Value,
) -> Result<Value> {
	expect_json_status(app, cookie, Method::PUT, uri, Some(body), StatusCode::OK)
		.await
}

pub(super) async fn request_json_ok_with_audit_compliance(
	app: &Router,
	cookie: &str,
	method: Method,
	uri: String,
	body: Value,
	reason: &str,
	category: Option<&str>,
) -> Result<Value> {
	let mut req = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.header("x-e2br3-reason-for-change", reason);
	if let Some(category) = category {
		req = req.header("x-e2br3-change-category", category);
	}
	let req = req.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let value = parse_json_or_raw(&bytes);
	assert_eq!(status, StatusCode::OK, "{value:?}");
	Ok(value)
}

pub(super) async fn create_sender_presave_via_api(
	app: &Router,
	cookie: &str,
	_authority: &str,
) -> Result<Uuid> {
	create_named_sender_presave_via_api(
		app,
		cookie,
		"legacy-unused",
		format!("REST Sender Details {}", Uuid::new_v4()),
		&format!("REST Sender Details Org {}", Uuid::new_v4()),
	)
	.await
}

pub(super) async fn create_named_sender_presave_via_api(
	app: &Router,
	cookie: &str,
	_authority: &str,
	_name: String,
	organization_name: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/senders".to_string(),
		json!({
			"data": {
				"sender_type": "1",
				"organization_name": organization_name,
				"country_code": "US",
				"email": "sender-details@example.com"
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_sender_presave_with_type_via_api(
	app: &Router,
	cookie: &str,
	sender_type: &str,
	_name: String,
	organization_name: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/senders".to_string(),
		json!({
			"data": {
				"sender_type": sender_type,
				"organization_name": organization_name
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_sender_gateway_via_api(
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

pub(super) async fn create_sender_responsible_person_via_api(
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

pub(super) async fn create_receiver_presave_via_api(
	app: &Router,
	cookie: &str,
	_authority: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/receivers".to_string(),
		json!({
			"data": {
				"receiver_type": "Regulatory Authority",
				"organization_name": format!("REST Receiver Details Org {}", Uuid::new_v4()),
				"receiver_identifier": format!("REC-{}", Uuid::new_v4())
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_receiver_consignee_via_api(
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

pub(super) async fn create_product_presave_via_api(
	app: &Router,
	cookie: &str,
	_authority: &str,
) -> Result<Uuid> {
	create_named_product_presave_via_api(
		app,
		cookie,
		"legacy-unused",
		format!("REST Product Details {}", Uuid::new_v4()),
		"REST Product Details",
	)
	.await
}

pub(super) async fn create_named_product_presave_via_api(
	app: &Router,
	cookie: &str,
	_authority: &str,
	_name: String,
	medicinal_product: &str,
) -> Result<Uuid> {
	let sender_id =
		create_sender_presave_via_api(app, cookie, "legacy-unused").await?;
	create_named_product_presave_for_sender_via_api(
		app,
		cookie,
		sender_id,
		_name,
		medicinal_product,
	)
	.await
}

pub(super) async fn create_named_product_presave_for_sender_via_api(
	app: &Router,
	cookie: &str,
	sender_id: Uuid,
	_name: String,
	medicinal_product: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/products".to_string(),
		json!({
			"data": {
				"sender_presave_id": sender_id,
				"product_id": format!("REST-PRODUCT-{}", Uuid::new_v4()),
				"medicinal_product": medicinal_product
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_product_presave_with_identity_for_sender_via_api(
	app: &Router,
	cookie: &str,
	sender_id: Uuid,
	product_id: Option<&str>,
	preapproval_ip_name: Option<&str>,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/products".to_string(),
		json!({
			"data": {
				"sender_presave_id": sender_id,
				"product_id": product_id,
				"preapproval_ip_name": preapproval_ip_name,
				"medicinal_product": "REST Product Identity"
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_product_substance_via_api(
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

pub(super) async fn create_study_presave_for_product_via_api(
	app: &Router,
	cookie: &str,
	product_id: Uuid,
	_authority: &str,
) -> Result<Uuid> {
	create_named_study_presave_for_product_via_api(
		app,
		cookie,
		product_id,
		"legacy-unused",
		format!("REST Study Details {}", Uuid::new_v4()),
		"REST Study Details",
	)
	.await
}

pub(super) async fn create_named_study_presave_for_product_via_api(
	app: &Router,
	cookie: &str,
	product_id: Uuid,
	_authority: &str,
	_name: String,
	study_name: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/studies".to_string(),
		json!({
			"data": {
				"product_presave_id": product_id,
				"study_name": study_name,
				"sponsor_study_number": format!("STUDY-{}", Uuid::new_v4()),
				"study_type_reaction": "1"
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_study_registration_number_via_api(
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

pub(super) async fn create_study_product_via_api(
	app: &Router,
	cookie: &str,
	study_id: Uuid,
	sequence_number: i32,
	product_id: Uuid,
	product_name: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/studies/{study_id}/products"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"product_presave_id": product_id,
				"product_name": product_name
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_study_reporter_via_api(
	app: &Router,
	cookie: &str,
	study_id: Uuid,
	sequence_number: i32,
	reporter_id: Uuid,
	organization: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		format!("/api/presaves/studies/{study_id}/reporters"),
		json!({
			"data": {
				"sequence_number": sequence_number,
				"reporter_presave_id": reporter_id,
				"reporter_organization": organization,
				"reporter_given_name": "Reporter",
				"reporter_qualification": "1"
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_named_reporter_presave_via_api(
	app: &Router,
	cookie: &str,
	_name: String,
	organization: &str,
) -> Result<Uuid> {
	let value = post_json_created(
		app,
		cookie,
		"/api/presaves/reporters".to_string(),
		json!({
			"data": {
				"reporter_given_name": "Reporter",
				"organization": organization,
				"qualification": "1",
				"primary_source_regulatory": "1"
			}
		}),
	)
	.await?;
	data_id(&value)
}

pub(super) async fn create_info_editor(
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
