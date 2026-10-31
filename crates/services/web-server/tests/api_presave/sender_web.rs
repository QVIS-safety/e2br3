use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn sender_details_accept_canonical_camel_case_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let sender_id = create_sender_presave_via_api(&app, &cookie, "fda").await?;

	let saved = put_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({ "data": { "rows": {
			"sender": { "organizationNameNotation": "ROWS-SENDER" },
			"gateways": [{
				"sequenceNumber": 1,
				"gatewayAuthority": "fda",
				"senderIdentifier": "ROWS-ID",
				"deleted": false
			}],
			"responsiblePersons": [{
				"sequenceNumber": 1,
				"personGivenName": "Mina",
				"deleted": false
			}]
		} } }),
	)
	.await?;

	assert_eq!(saved["data"]["rows"]["sender"]["id"], sender_id.to_string());
	assert_eq!(
		saved["data"]["rows"]["gateways"][0]["senderIdentifier"],
		"ROWS-ID"
	);
	assert_eq!(
		saved["data"]["rows"]["responsiblePersons"][0]["personGivenName"],
		"Mina"
	);
	assert!(saved["data"].get("parent").is_none());
	for legacy_data in [
		json!({ "parent": {} }),
		json!({ "responsible_persons": [] }),
		json!({ "rows": { "gateways": [{ "_delete": true }] } }),
	] {
		let (status, value) = request_json(
			&app,
			&cookie,
			Method::PUT,
			format!("/api/presaves/senders/{sender_id}/details"),
			Some(json!({ "data": legacy_data })),
		)
		.await?;
		assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{value:?}");
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn sender_list_includes_responsible_person_rows() -> Result<()> {
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
				"organizationName": format!("List Sender {}", Uuid::new_v4())
			},
			"gateways": [],
			"responsiblePersons": [{
				"sequenceNumber": 1,
				"personGivenName": "Mina",
				"deleted": false
			}]
		} } }),
	)
	.await?;
	let sender_id = created["data"]["rows"]["sender"]["id"]
		.as_str()
		.expect("created sender id");

	let listed =
		get_json_ok(&app, &cookie, "/api/presaves/senders".to_string()).await?;
	let sender = listed["data"]
		.as_array()
		.expect("sender list")
		.iter()
		.find(|row| row["rows"]["sender"]["id"] == sender_id)
		.expect("created sender in list");
	assert_eq!(
		sender["rows"]["responsiblePersons"][0]["personGivenName"],
		"Mina"
	);

	Ok(())
}

#[tokio::test]
async fn test_sender_presave_parent_does_not_store_person_or_department_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let parent_without_given_name = json!({
		"data": { "rows": {
			"sender": {
				"senderType": "1",
				"organizationName": format!("Sender Without Parent Given Org {}", Uuid::new_v4())
			}, "gateways": [], "responsiblePersons": []
		} }
	});
	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/senders".to_string(),
		Some(parent_without_given_name),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(
		value["data"]["rows"]["sender"]
			.get("personGivenName")
			.is_none(),
		"sender parent response must not expose person_given_name: {value:?}",
	);
	assert!(
		value["data"]["rows"]["sender"].get("department").is_none(),
		"sender parent response must not expose department: {value:?}",
	);

	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;
	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_id}/details"),
		Some(json!({
			"data": { "rows": {
				"sender": {
					"senderType": "1",
					"organizationName": format!("Updated Sender Org {}", Uuid::new_v4())
				},
				"responsiblePersons": [{
					"sequenceNumber": 1,
					"department": "Safety Ops",
					"personGivenName": "Ada"
				}]
			} }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]["rows"]["sender"].get("personGivenName").is_none(),
		"sender details parent response must not expose person_given_name: {value:?}",
	);
	assert!(
		value["data"]["rows"]["sender"].get("department").is_none(),
		"sender details parent response must not expose department: {value:?}",
	);
	assert_eq!(
		value["data"]["rows"]["responsiblePersons"][0]["department"].as_str(),
		Some("Safety Ops"),
	);
	assert_eq!(
		value["data"]["rows"]["responsiblePersons"][0]["personGivenName"].as_str(),
		Some("Ada"),
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_presave_rejects_duplicate_active_identity() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let organization_name = format!("Duplicate Sender Org {}", Uuid::new_v4());

	let sender_id = create_sender_presave_with_type_via_api(
		&app,
		&admin_cookie,
		"1",
		format!("Duplicate Sender {}", Uuid::new_v4()),
		&organization_name,
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/senders".to_string(),
		Some(json!({ "data": { "rows": {
			"sender": { "senderType": "1", "organizationName": organization_name },
			"gateways": [], "responsiblePersons": []
		} } })),
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert!(
		value
			.to_string()
			.contains("sender presave duplicate identity"),
		"unexpected duplicate sender body: {value:?}"
	);

	let different_type_id = create_sender_presave_with_type_via_api(
		&app,
		&admin_cookie,
		"2",
		format!("Different Type Sender {}", Uuid::new_v4()),
		&organization_name,
	)
	.await?;
	assert_ne!(sender_id, different_type_id);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::DELETE,
		format!("/api/presaves/senders/{sender_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");

	let reused_id = create_sender_presave_with_type_via_api(
		&app,
		&admin_cookie,
		"1",
		format!("Reused Sender {}", Uuid::new_v4()),
		&organization_name,
	)
	.await?;
	assert_ne!(sender_id, reused_id);

	Ok(())
}

#[serial]
#[tokio::test]
async fn info_update_audit_reason_records_sender_presave_reason() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;
	let reason = "Edited Data: Corrected sender organization";
	let organization_name = format!("Audit Reason Sender Org {}", Uuid::new_v4());

	let category = "Edited Data";
	request_json_ok_with_audit_compliance(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({ "data": { "rows": {
			"sender": { "organizationName": organization_name }
		} } }),
		reason,
		Some(category),
	)
	.await?;

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let recorded_compliance = dbx
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>, Option<String>)>(
				r#"
				SELECT reason_for_change, change_category
				FROM audit_logs
				WHERE table_name = 'sender_presaves'
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'organization_name'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(sender_id),
		)
		.await?;
	dbx.rollback_txn().await?;

	let (recorded_reason, recorded_category) =
		recorded_compliance.unwrap_or_default();
	assert_eq!(recorded_reason, Some(reason.to_string()));
	assert_eq!(recorded_category, Some(category.to_string()));

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
	assert_eq!(
		details["data"]["rows"]["sender"]["id"],
		sender_id.to_string()
	);
	assert_eq!(
		details["data"]["rows"]["gateways"][0]["id"],
		gateway_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({
			"data": { "rows": {
				"sender": {
					"organizationNameNotation": "REST notation"
				},
				"gateways": [
					{
						"id": gateway_id,
						"sequenceNumber": 2,
						"gatewayAuthority": "mfds",
						"senderIdentifier": "SENDER-2"
					},
					{
						"sequenceNumber": 3,
						"gatewayAuthority": "fda",
						"senderIdentifier": "SENDER-3"
					}
				],
				"responsiblePersons": [
					{
						"sequenceNumber": 1,
						"department": "Safety",
						"personGivenName": "Ari",
						"personFamilyName": "Kim"
					}
				]
			} }
		}),
	)
	.await?;
	assert!(
		saved["data"]["rows"]["sender"].get("comments").is_none(),
		"{saved:?}"
	);
	assert_eq!(
		saved["data"]["rows"]["gateways"].as_array().unwrap().len(),
		2
	);
	assert_eq!(
		saved["data"]["rows"]["responsiblePersons"]
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
	assert!(
		persisted["data"]["rows"]["sender"]
			.get("comments")
			.is_none(),
		"{persisted:?}"
	);
	assert_eq!(
		persisted["data"]["rows"]["sender"]["organizationNameNotation"].as_str(),
		Some("REST notation"),
		"{persisted:?}"
	);
	let gateways = persisted["data"]["rows"]["gateways"].as_array().unwrap();
	assert_eq!(gateways.len(), 2, "{persisted:?}");
	let updated_gateway = gateways
		.iter()
		.find(|row| row["id"].as_str() == Some(&gateway_id.to_string()))
		.ok_or("missing updated gateway")?;
	assert_eq!(
		updated_gateway["senderIdentifier"].as_str(),
		Some("SENDER-2")
	);
	assert_eq!(updated_gateway["gatewayAuthority"].as_str(), Some("mfds"));
	assert_eq!(updated_gateway["sequenceNumber"].as_i64(), Some(2));
	let created_gateway = gateways
		.iter()
		.find(|row| row["senderIdentifier"].as_str() == Some("SENDER-3"))
		.ok_or("missing created gateway")?;
	assert_eq!(created_gateway["gatewayAuthority"].as_str(), Some("fda"));

	let responsible_persons = persisted["data"]["rows"]["responsiblePersons"]
		.as_array()
		.unwrap();
	let responsible_person = responsible_persons
		.iter()
		.find(|row| row["personGivenName"].as_str() == Some("Ari"))
		.ok_or("missing responsible person")?;
	assert_eq!(responsible_person["department"].as_str(), Some("Safety"));
	assert_eq!(responsible_person["personFamilyName"].as_str(), Some("Kim"));

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
			"data": { "rows": {
				"sender": { "organizationName": "must roll back" },
				"gateways": [{
					"sequenceNumber": 1,
					"gatewayAuthority": "invalid",
					"senderIdentifier": "INVALID-GATEWAY"
				}]
			} }
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
	assert_ne!(
		persisted["data"]["rows"]["sender"]["organizationName"].as_str(),
		Some("must roll back"),
		"{persisted:?}"
	);
	let gateways = persisted["data"]["rows"]["gateways"].as_array().unwrap();
	assert!(
		!gateways
			.iter()
			.any(|row| row["senderIdentifier"].as_str() == Some("INVALID-GATEWAY")),
		"{persisted:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_presave_direct_child_delete_soft_deletes_details_rows(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "ich").await?;
	let gateway_id =
		create_sender_gateway_via_api(&app, &admin_cookie, sender_id, 1, "DELETE")
			.await?;
	let responsible_id = create_sender_responsible_person_via_api(
		&app,
		&admin_cookie,
		sender_id,
		1,
		"Ari",
	)
	.await?;

	for uri in [
		format!("/api/presaves/senders/{sender_id}/gateways/{gateway_id}"),
		format!(
			"/api/presaves/senders/{sender_id}/responsible-persons/{responsible_id}"
		),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::DELETE, uri, None).await?;
		assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");
	}

	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	let gateways = after_delete["data"]["rows"]["gateways"].as_array().unwrap();
	let responsible_persons = after_delete["data"]["rows"]["responsiblePersons"]
		.as_array()
		.unwrap();
	let deleted_gateway = gateways
		.iter()
		.find(|row| row["id"].as_str() == Some(&gateway_id.to_string()))
		.ok_or("missing direct-deleted gateway")?;
	assert_eq!(deleted_gateway["deleted"].as_bool(), Some(true));
	assert_eq!(deleted_gateway["senderIdentifier"].as_str(), Some("DELETE"));
	assert_eq!(
		deleted_gateway["routingIdentifier"].as_str(),
		Some("ROUTE-DELETE")
	);
	let deleted_responsible_person = responsible_persons
		.iter()
		.find(|row| row["id"].as_str() == Some(&responsible_id.to_string()))
		.ok_or("missing direct-deleted responsible person")?;
	assert_eq!(deleted_responsible_person["deleted"].as_bool(), Some(true));
	assert_eq!(
		deleted_responsible_person["personGivenName"].as_str(),
		Some("Ari")
	);
	assert_eq!(
		deleted_responsible_person["personFamilyName"].as_str(),
		Some("Kim")
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({
			"data": { "rows": {
				"gateways": [{ "id": gateway_id, "senderIdentifier": "DELETE" }],
				"responsiblePersons": [{ "id": responsible_id, "personGivenName": "Ari" }]
			} }
		}),
	)
	.await?;
	let after_deleted_omission = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	for (rows_key, child_id) in [
		("gateways", gateway_id),
		("responsiblePersons", responsible_id),
	] {
		let child = after_deleted_omission["data"]["rows"][rows_key]
			.as_array()
			.unwrap()
			.iter()
			.find(|row| row["id"].as_str() == Some(&child_id.to_string()))
			.ok_or("missing child after omitted deleted flag")?;
		assert_eq!(child["deleted"].as_bool(), Some(true));
	}

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({
			"data": { "rows": {
				"gateways": [{ "id": gateway_id, "deleted": false }],
				"responsiblePersons": [{ "id": responsible_id, "deleted": false }]
			} }
		}),
	)
	.await?;
	let after_restore = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	for (rows_key, child_id) in [
		("gateways", gateway_id),
		("responsiblePersons", responsible_id),
	] {
		let child = after_restore["data"]["rows"][rows_key]
			.as_array()
			.unwrap()
			.iter()
			.find(|row| row["id"].as_str() == Some(&child_id.to_string()))
			.ok_or("missing restored child")?;
		assert_eq!(child["deleted"].as_bool(), Some(false));
	}

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
		json!({
			"data": { "rows": {
				"gateways": [{ "id": gateway_id, "deleted": true }],
				"responsiblePersons": [{ "id": responsible_id, "deleted": true }]
			} }
		}),
	)
	.await?;
	let after_redelete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	for (rows_key, child_id) in [
		("gateways", gateway_id),
		("responsiblePersons", responsible_id),
	] {
		let child = after_redelete["data"]["rows"][rows_key]
			.as_array()
			.unwrap()
			.iter()
			.find(|row| row["id"].as_str() == Some(&child_id.to_string()))
			.ok_or("missing re-deleted child")?;
		assert_eq!(child["deleted"].as_bool(), Some(true));
	}

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
		json!({ "data": { "rows": { "sender": { "organizationName": "omit children" } } } }),
	)
	.await?;
	let after_omit = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	let gateways = after_omit["data"]["rows"]["gateways"].as_array().unwrap();
	let responsible_persons = after_omit["data"]["rows"]["responsiblePersons"]
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
		json!({ "data": { "rows": { "gateways": [], "responsiblePersons": [] } } }),
	)
	.await?;
	let after_empty = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	let gateways = after_empty["data"]["rows"]["gateways"].as_array().unwrap();
	let responsible_persons = after_empty["data"]["rows"]["responsiblePersons"]
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
		json!({
			"data": { "rows": {
				"gateways": [{ "id": gateway_delete_id, "deleted": true }],
				"responsiblePersons": [{ "id": responsible_id, "deleted": true }]
			} }
		}),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/senders/{sender_id}/details"),
	)
	.await?;
	let gateways = after_delete["data"]["rows"]["gateways"].as_array().unwrap();
	let responsible_persons = after_delete["data"]["rows"]["responsiblePersons"]
		.as_array()
		.unwrap();
	assert_eq!(gateways.len(), 2);
	assert_eq!(responsible_persons.len(), 1);
	let deleted_gateway = gateways
		.iter()
		.find(|row| row["id"].as_str() == Some(&gateway_delete_id.to_string()))
		.ok_or("missing deleted gateway")?;
	assert_eq!(deleted_gateway["deleted"].as_bool(), Some(true));
	assert_eq!(deleted_gateway["senderIdentifier"].as_str(), Some("DELETE"));
	assert_eq!(
		deleted_gateway["routingIdentifier"].as_str(),
		Some("ROUTE-DELETE")
	);
	assert!(
		gateways
			.iter()
			.any(|row| row["id"].as_str() == Some(&gateway_keep_id.to_string())),
		"{after_delete:?}"
	);
	let kept_gateway = gateways
		.iter()
		.find(|row| row["id"].as_str() == Some(&gateway_keep_id.to_string()))
		.ok_or("missing kept gateway")?;
	assert_eq!(kept_gateway["deleted"].as_bool(), Some(false));
	let deleted_responsible_person = responsible_persons
		.iter()
		.find(|row| row["id"].as_str() == Some(&responsible_id.to_string()))
		.ok_or("missing deleted responsible person")?;
	assert_eq!(deleted_responsible_person["deleted"].as_bool(), Some(true));
	assert_eq!(
		deleted_responsible_person["personGivenName"].as_str(),
		Some("Ari")
	);
	assert_eq!(
		deleted_responsible_person["personFamilyName"].as_str(),
		Some("Kim")
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
		Some(json!({ "data": { "rows": { "gateways": [{ "deleted": true }] } } })),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(
			json!({ "data": { "rows": { "responsiblePersons": [{ "deleted": true }] } } }),
		),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(json!({ "data": { "rows": { "gateways": [{ "id": gateway_b, "deleted": true }] } } })),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(json!({
			"data": { "rows": {
				"responsiblePersons": [{ "id": responsible_b, "deleted": true }]
			} }
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
			"data": { "rows": {
				"gateways": [{
					"id": gateway_b,
					"sequenceNumber": 2,
					"gatewayAuthority": "fda",
					"senderIdentifier": "WRONG-PARENT-UPDATE"
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
		format!("/api/presaves/senders/{sender_a}/details"),
		Some(json!({
			"data": { "rows": {
				"responsiblePersons": [{
					"id": responsible_b,
					"sequenceNumber": 2,
					"department": "Wrong Parent",
					"personGivenName": "Wrong",
					"personFamilyName": "Parent"
				}]
			} }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");

	Ok(())
}
