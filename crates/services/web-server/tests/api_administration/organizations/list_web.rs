use crate::administration_support::request_json;
use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	system_user_id, Result,
};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SYSTEM_ADMIN;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn system_admin_lists_filtered_sorted_organizations_across_tenants(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let first = seed_org_with_all_roles(&mm).await?;
	let admin = insert_user(
		&mm,
		first.org_id,
		ROLE_SYSTEM_ADMIN,
		system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let cookie = cookie_header(
		&generate_web_token(&admin.email, admin.token_salt)?.to_string(),
	);
	let app = web_server::app(mm);
	let marker = Uuid::new_v4();
	let mut created_ids = Vec::new();

	for (name, active) in [
		(format!("Gap A {marker}"), true),
		(format!("Gap B {marker}"), false),
	] {
		let (status, created) = request_json(&app, "POST", &cookie, "/api/organizations", Some(json!({
			"data": { "name": name, "type": "CRO", "contact_email": format!("{marker}-{active}@example.com") }
		}))).await?;
		assert_eq!(status, StatusCode::CREATED, "{created:?}");
		created_ids.push(
			created["data"]["id"]
				.as_str()
				.ok_or("missing organization id")?
				.to_string(),
		);
		if !active {
			let id = created["data"]["id"]
				.as_str()
				.ok_or("missing organization id")?;
			let (status, body) = request_json(
				&app,
				"PUT",
				&cookie,
				format!("/api/organizations/{id}"),
				Some(json!({"data": {"active": false}})),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{body:?}");
		}
	}

	let uri = format!("/api/organizations?filters[name][%24contains]={marker}&filters[active][%24eq]=true&list_options[order_bys]=name&list_options[limit]=1");
	let (status, body) = request_json(&app, "GET", &cookie, uri, None).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let rows = body["data"]
		.as_array()
		.ok_or("missing organizations array")?;
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0]["name"], json!(format!("Gap A {marker}")));

	let (status, body) = request_json(
		&app,
		"GET",
		&cookie,
		format!("/api/organizations?filters[name][%24contains]={marker}&list_options[order_bys]=name&list_options[limit]=2"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let ids = body["data"]
		.as_array()
		.ok_or("missing organizations array")?
		.iter()
		.map(|row| row["id"].as_str().ok_or("missing organization id"))
		.collect::<std::result::Result<Vec<_>, _>>()?;
	assert_eq!(
		ids,
		created_ids.iter().map(String::as_str).collect::<Vec<_>>()
	);
	assert!(created_ids.iter().all(|id| id != &first.org_id.to_string()));
	Ok(())
}
