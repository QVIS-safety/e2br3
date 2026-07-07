use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn section_presave_receiver_details_contract_includes_routes() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let receiver_id =
		create_receiver_presave_via_api(&app, &admin_cookie, "fda").await?;

	let value = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/receivers/{receiver_id}/details"),
		json!({
			"data": {
				"children": {
					"routes": [{
						"sequence_number": 1,
						"authority": "fda",
						"receiver_label": "FDA(CBER IND)",
						"batch_receiver_identifier": "CBER_IND",
						"message_receiver_identifier": "CBER_IND",
						"condition_page": "CI",
						"condition_field_code": "FDA_REPORT_TYPE",
						"condition_operator": "Equal",
						"condition_value_code": "3",
						"condition_value_label": "CBER IND"
					}]
				}
			}
		}),
	)
	.await?;

	assert_eq!(
		value["data"]["children"]["routes"][0]["receiver_label"],
		"FDA(CBER IND)"
	);
	assert_eq!(
		value["data"]["children"]["routes"][0]["condition_value_code"],
		"3"
	);

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
