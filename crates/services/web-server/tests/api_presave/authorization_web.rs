use super::helpers::*;
use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_users, system_user_id,
	Result, TEST_CUSTOM_MANAGER_ROLE,
};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

struct PresaveFixture {
	sender_id: Uuid,
	gateway_id: Uuid,
	responsible_id: Uuid,
	receiver_id: Uuid,
	consignee_id: Uuid,
	route_id: Uuid,
	product_id: Uuid,
	substance_id: Uuid,
	reporter_id: Uuid,
	study_id: Uuid,
	registration_id: Uuid,
	study_product_id: Uuid,
	study_reporter_id: Uuid,
	fda_ind_id: Uuid,
	narrative_id: Uuid,
}

async fn create_fixture(app: &axum::Router, cookie: &str) -> Result<PresaveFixture> {
	let sender_id = create_sender_presave_via_api(app, cookie, "fda").await?;
	let gateway_id =
		create_sender_gateway_via_api(app, cookie, sender_id, 1, "AUTHZ").await?;
	let responsible_id = create_sender_responsible_person_via_api(
		app,
		cookie,
		sender_id,
		1,
		"Authorization",
	)
	.await?;
	let receiver_id = create_receiver_presave_via_api(app, cookie, "fda").await?;
	let consignee_id = create_receiver_consignee_via_api(
		app,
		cookie,
		receiver_id,
		1,
		"Authorization Consignee",
	)
	.await?;
	let receiver_details = put_json_ok(
		app,
		cookie,
		format!("/api/presaves/receivers/{receiver_id}/details"),
		json!({ "data": { "rows": { "routes": [{
			"sequenceNumber": 1,
			"authority": "fda",
			"receiverLabel": "Authorization route"
		}] } } }),
	)
	.await?;
	let route_id = Uuid::parse_str(
		receiver_details["data"]["rows"]["routes"][0]["id"]
			.as_str()
			.ok_or("missing receiver route id")?,
	)?;
	let product_id = create_named_product_presave_for_sender_via_api(
		app,
		cookie,
		sender_id,
		format!("Authorization Product {}", Uuid::new_v4()),
		"Authorization Product",
	)
	.await?;
	let substance_id = create_product_active_substance_via_api(
		app,
		cookie,
		product_id,
		1,
		"Authorization Substance",
	)
	.await?;
	let reporter_id = create_named_reporter_presave_via_api(
		app,
		cookie,
		format!("Authorization Reporter {}", Uuid::new_v4()),
		"Authorization Reporter Org",
	)
	.await?;
	let study_id =
		create_study_presave_for_product_via_api(app, cookie, product_id, "fda")
			.await?;
	let registration_id = create_study_registration_number_via_api(
		app,
		cookie,
		study_id,
		1,
		"AUTHZ-REG",
	)
	.await?;
	let study_product_id = create_study_product_via_api(
		app,
		cookie,
		study_id,
		1,
		product_id,
		"Authorization Study Product",
	)
	.await?;
	let study_reporter_id = create_study_reporter_via_api(
		app,
		cookie,
		study_id,
		1,
		reporter_id,
		"Authorization Reporter Org",
	)
	.await?;
	let study_details = put_json_ok(
		app,
		cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({ "data": { "rows": { "fdaCrossReportedInds": [{
			"sequenceNumber": 1,
			"indNumber": "AUTHZ-IND"
		}] } } }),
	)
	.await?;
	let fda_ind_id = Uuid::parse_str(
		study_details["data"]["rows"]["fdaCrossReportedInds"][0]["id"]
			.as_str()
			.ok_or("missing FDA IND id")?,
	)?;
	let narrative = post_json_created(
		app,
		cookie,
		"/api/presaves/narratives".to_string(),
		json!({ "data": { "rows": { "narrative": {
			"caseNarrative": "Authorization narrative"
		} } } }),
	)
	.await?;
	let narrative_id = data_rows_id(&narrative, "narrative")?;

	Ok(PresaveFixture {
		sender_id,
		gateway_id,
		responsible_id,
		receiver_id,
		consignee_id,
		route_id,
		product_id,
		substance_id,
		reporter_id,
		study_id,
		registration_id,
		study_product_id,
		study_reporter_id,
		fda_ind_id,
		narrative_id,
	})
}

async fn audit_count_for_ids(mm: &ModelManager, ids: &[Uuid]) -> Result<i64> {
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let count = sqlx::query_scalar::<_, i64>(
		"SELECT count(*) FROM audit_logs WHERE record_id = ANY($1)",
	)
	.bind(ids.to_vec())
	.fetch_one(&mut *tx)
	.await?;
	tx.rollback().await?;
	Ok(count)
}

#[serial]
#[tokio::test]
async fn presave_read_only_and_cross_org_roles_cannot_mutate_any_resource(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let owner = seed_org_with_users(&mm, "ownerpwd", "ownerviewpwd").await?;
	let outsider =
		seed_org_with_users(&mm, "outsiderpwd", "outsiderviewpwd").await?;
	let owner_token =
		generate_web_token(&owner.admin.email, owner.admin.token_salt)?;
	let owner_cookie = cookie_header(&owner_token.to_string());
	let outsider_token =
		generate_web_token(&outsider.admin.email, outsider.admin.token_salt)?;
	let outsider_cookie = cookie_header(&outsider_token.to_string());
	let viewer_token =
		generate_web_token(&owner.viewer.email, owner.viewer.token_salt)?;
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let manager = insert_user(
		&mm,
		owner.org_id,
		TEST_CUSTOM_MANAGER_ROLE,
		system_user_id(),
		Some("managerpwd"),
	)
	.await?;
	let manager_token = generate_web_token(&manager.email, manager.token_salt)?;
	let manager_cookie = cookie_header(&manager_token.to_string());
	let app = web_server::app(mm.clone());
	let fixture = create_fixture(&app, &owner_cookie).await?;

	let read_uris = [
		format!("/api/presaves/senders/{}", fixture.sender_id),
		format!(
			"/api/presaves/senders/{}/gateways/{}",
			fixture.sender_id, fixture.gateway_id
		),
		format!(
			"/api/presaves/senders/{}/responsible-persons/{}",
			fixture.sender_id, fixture.responsible_id
		),
		format!("/api/presaves/receivers/{}", fixture.receiver_id),
		format!(
			"/api/presaves/receivers/{}/consignees/{}",
			fixture.receiver_id, fixture.consignee_id
		),
		format!("/api/presaves/receivers/{}/details", fixture.receiver_id),
		format!("/api/presaves/products/{}", fixture.product_id),
		format!(
			"/api/presaves/products/{}/active-substances/{}",
			fixture.product_id, fixture.substance_id
		),
		format!("/api/presaves/reporters/{}", fixture.reporter_id),
		format!("/api/presaves/studies/{}", fixture.study_id),
		format!(
			"/api/presaves/studies/{}/registration-numbers/{}",
			fixture.study_id, fixture.registration_id
		),
		format!(
			"/api/presaves/studies/{}/products/{}",
			fixture.study_id, fixture.study_product_id
		),
		format!(
			"/api/presaves/studies/{}/reporters/{}",
			fixture.study_id, fixture.study_reporter_id
		),
		format!("/api/presaves/studies/{}/details", fixture.study_id),
		format!("/api/presaves/narratives/{}", fixture.narrative_id),
	];
	for uri in &read_uris {
		let (status, value) =
			request_json(&app, &manager_cookie, Method::GET, uri.clone(), None)
				.await?;
		assert_eq!(status, StatusCode::OK, "manager GET {uri}: {value:?}");
		for (label, cookie, expected) in [
			("case-only viewer", &viewer_cookie, StatusCode::FORBIDDEN),
			(
				"cross-organization admin",
				&outsider_cookie,
				StatusCode::NOT_FOUND,
			),
		] {
			let (status, value) =
				request_json(&app, cookie, Method::GET, uri.clone(), None).await?;
			assert_eq!(status, expected, "{label} GET {uri}: {value:?}");
		}
	}
	assert!(get_json_ok(
		&app,
		&manager_cookie,
		format!("/api/presaves/receivers/{}/details", fixture.receiver_id),
	)
	.await?["data"]["rows"]["routes"]
		.as_array()
		.ok_or("missing receiver routes")?
		.iter()
		.any(|row| row["id"].as_str() == Some(&fixture.route_id.to_string())));
	assert!(get_json_ok(
		&app,
		&manager_cookie,
		format!("/api/presaves/studies/{}/details", fixture.study_id),
	)
	.await?["data"]["rows"]["fdaCrossReportedInds"]
		.as_array()
		.ok_or("missing FDA IND rows")?
		.iter()
		.any(|row| row["id"].as_str() == Some(&fixture.fda_ind_id.to_string())));

	let owner_snapshots = [
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/senders/{}/details", fixture.sender_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/receivers/{}/details", fixture.receiver_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/products/{}/details", fixture.product_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/studies/{}/details", fixture.study_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/reporters/{}", fixture.reporter_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/narratives/{}", fixture.narrative_id),
		)
		.await?,
	];
	let ids = [
		fixture.sender_id,
		fixture.gateway_id,
		fixture.responsible_id,
		fixture.receiver_id,
		fixture.consignee_id,
		fixture.route_id,
		fixture.product_id,
		fixture.substance_id,
		fixture.reporter_id,
		fixture.study_id,
		fixture.registration_id,
		fixture.study_product_id,
		fixture.study_reporter_id,
		fixture.fda_ind_id,
		fixture.narrative_id,
	];
	let audit_before = audit_count_for_ids(&mm, &ids).await?;

	let mutations: Vec<(Method, String, Value)> = vec![
		(
			Method::PATCH,
			format!("/api/presaves/senders/{}", fixture.sender_id),
			json!({"data":{"city":"Denied"}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/senders/{}/gateways/{}",
				fixture.sender_id, fixture.gateway_id
			),
			json!({"data":{"sender_identifier":"DENIED"}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/senders/{}/responsible-persons/{}",
				fixture.sender_id, fixture.responsible_id
			),
			json!({"data":{"department":"Denied"}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/receivers/{}", fixture.receiver_id),
			json!({"data":{"description":"Denied"}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/receivers/{}/consignees/{}",
				fixture.receiver_id, fixture.consignee_id
			),
			json!({"data":{"name":"Denied"}}),
		),
		(
			Method::PUT,
			format!("/api/presaves/receivers/{}/details", fixture.receiver_id),
			json!({"data":{"rows":{"routes":[{"id":fixture.route_id,"receiverLabel":"Denied"}]}}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/products/{}", fixture.product_id),
			json!({"data":{"medicinalProduct":"Denied"}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/products/{}/active-substances/{}",
				fixture.product_id, fixture.substance_id
			),
			json!({"data":{"substance_name":"Denied"}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/reporters/{}", fixture.reporter_id),
			json!({"data":{"rows":{"reporter":{"organization":"Denied"}}}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/studies/{}", fixture.study_id),
			json!({"data":{"studyName":"Denied"}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/studies/{}/registration-numbers/{}",
				fixture.study_id, fixture.registration_id
			),
			json!({"data":{"registration_number":"DENIED"}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/studies/{}/products/{}",
				fixture.study_id, fixture.study_product_id
			),
			json!({"data":{"product_name":"Denied"}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/studies/{}/reporters/{}",
				fixture.study_id, fixture.study_reporter_id
			),
			json!({"data":{"reporter_organization":"Denied"}}),
		),
		(
			Method::PUT,
			format!("/api/presaves/studies/{}/details", fixture.study_id),
			json!({"data":{"rows":{"fdaCrossReportedInds":[{"id":fixture.fda_ind_id,"indNumber":"DENIED"}]}}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/narratives/{}", fixture.narrative_id),
			json!({"data":{"rows":{"narrative":{"caseNarrative":"Denied"}}}}),
		),
		(
			Method::DELETE,
			format!("/api/presaves/senders/{}", fixture.sender_id),
			Value::Null,
		),
		(
			Method::DELETE,
			format!(
				"/api/presaves/senders/{}/gateways/{}",
				fixture.sender_id, fixture.gateway_id
			),
			Value::Null,
		),
		(
			Method::DELETE,
			format!(
				"/api/presaves/senders/{}/responsible-persons/{}",
				fixture.sender_id, fixture.responsible_id
			),
			Value::Null,
		),
		(
			Method::DELETE,
			format!("/api/presaves/receivers/{}", fixture.receiver_id),
			Value::Null,
		),
		(
			Method::DELETE,
			format!(
				"/api/presaves/receivers/{}/consignees/{}",
				fixture.receiver_id, fixture.consignee_id
			),
			Value::Null,
		),
		(
			Method::DELETE,
			format!("/api/presaves/products/{}", fixture.product_id),
			Value::Null,
		),
		(
			Method::DELETE,
			format!(
				"/api/presaves/products/{}/active-substances/{}",
				fixture.product_id, fixture.substance_id
			),
			Value::Null,
		),
		(
			Method::DELETE,
			format!("/api/presaves/reporters/{}", fixture.reporter_id),
			Value::Null,
		),
		(
			Method::DELETE,
			format!("/api/presaves/studies/{}", fixture.study_id),
			Value::Null,
		),
		(
			Method::DELETE,
			format!(
				"/api/presaves/studies/{}/registration-numbers/{}",
				fixture.study_id, fixture.registration_id
			),
			Value::Null,
		),
		(
			Method::DELETE,
			format!(
				"/api/presaves/studies/{}/products/{}",
				fixture.study_id, fixture.study_product_id
			),
			Value::Null,
		),
		(
			Method::DELETE,
			format!(
				"/api/presaves/studies/{}/reporters/{}",
				fixture.study_id, fixture.study_reporter_id
			),
			Value::Null,
		),
		(
			Method::DELETE,
			format!("/api/presaves/narratives/{}", fixture.narrative_id),
			Value::Null,
		),
		(
			Method::PATCH,
			format!("/api/presaves/senders/{}", fixture.sender_id),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/senders/{}/gateways/{}",
				fixture.sender_id, fixture.gateway_id
			),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/senders/{}/responsible-persons/{}",
				fixture.sender_id, fixture.responsible_id
			),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/receivers/{}", fixture.receiver_id),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/products/{}", fixture.product_id),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/reporters/{}", fixture.reporter_id),
			json!({"data":{"rows":{"reporter":{"deleted":false}}}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/studies/{}", fixture.study_id),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/studies/{}/registration-numbers/{}",
				fixture.study_id, fixture.registration_id
			),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/studies/{}/products/{}",
				fixture.study_id, fixture.study_product_id
			),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!(
				"/api/presaves/studies/{}/reporters/{}",
				fixture.study_id, fixture.study_reporter_id
			),
			json!({"data":{"deleted":false}}),
		),
		(
			Method::PATCH,
			format!("/api/presaves/narratives/{}", fixture.narrative_id),
			json!({"data":{"rows":{"narrative":{"deleted":false}}}}),
		),
		(
			Method::PUT,
			format!("/api/presaves/receivers/{}/details", fixture.receiver_id),
			json!({"data":{"rows":{"routes":[{"id":fixture.route_id,"deleted":true}]}}}),
		),
		(
			Method::PUT,
			format!("/api/presaves/studies/{}/details", fixture.study_id),
			json!({"data":{"rows":{"fdaCrossReportedInds":[{"id":fixture.fda_ind_id,"deleted":true}]}}}),
		),
		(
			Method::PUT,
			format!("/api/presaves/studies/{}/details", fixture.study_id),
			json!({"data":{"rows":{"fdaCrossReportedInds":[{"id":fixture.fda_ind_id,"deleted":false}]}}}),
		),
	];
	for (method, uri, body) in mutations {
		for (label, cookie, expected) in [
			("read-only manager", &manager_cookie, StatusCode::FORBIDDEN),
			("case-only viewer", &viewer_cookie, StatusCode::FORBIDDEN),
			(
				"cross-organization admin",
				&outsider_cookie,
				StatusCode::NOT_FOUND,
			),
		] {
			let (status, value) = request_json(
				&app,
				cookie,
				method.clone(),
				uri.clone(),
				(method != Method::DELETE).then(|| body.clone()),
			)
			.await?;
			assert_eq!(status, expected, "{label} {uri}: {value:?}");
		}
	}

	let owner_after = [
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/senders/{}/details", fixture.sender_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/receivers/{}/details", fixture.receiver_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/products/{}/details", fixture.product_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/studies/{}/details", fixture.study_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/reporters/{}", fixture.reporter_id),
		)
		.await?,
		get_json_ok(
			&app,
			&owner_cookie,
			format!("/api/presaves/narratives/{}", fixture.narrative_id),
		)
		.await?,
	];
	for (before, after) in owner_snapshots.iter().zip(owner_after.iter()) {
		assert_eq!(after["data"], before["data"]);
	}
	assert_eq!(audit_count_for_ids(&mm, &ids).await?, audit_before);

	Ok(())
}
