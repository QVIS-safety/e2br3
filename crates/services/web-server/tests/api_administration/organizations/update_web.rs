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
async fn update_normalizes_type_persists_and_audits_while_invalid_input_is_unchanged(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin = insert_user(
		&mm,
		seed.org_id,
		ROLE_SYSTEM_ADMIN,
		system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let cookie = cookie_header(
		&generate_web_token(&admin.email, admin.token_salt)?.to_string(),
	);
	let app = web_server::app(mm.clone());
	let marker = Uuid::new_v4();
	let (status, created) = request_json(
		&app,
		"POST",
		&cookie,
		"/api/organizations",
		Some(json!({"data": {"name": format!("Update {marker}"), "type": "CRO"}})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{created:?}");
	let id = created["data"]["id"]
		.as_str()
		.ok_or("missing organization id")?;

	let (status, updated) = request_json(
		&app,
		"PUT",
		&cookie,
		format!("/api/organizations/{id}"),
		Some(json!({"data": {"type": " Pharmaceutical Company "}})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{updated:?}");
	assert_eq!(updated["data"]["type"], "pharmaceutical_company");
	let (status, fetched) = request_json(
		&app,
		"GET",
		&cookie,
		format!("/api/organizations/{id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{fetched:?}");
	assert_eq!(fetched["data"]["type"], "pharmaceutical_company");

	let (status, rejected) = request_json(
		&app,
		"PUT",
		&cookie,
		format!("/api/organizations/{id}"),
		Some(json!({"data": {"type": "unknown"}})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{rejected:?}");
	let (status, fetched) = request_json(
		&app,
		"GET",
		&cookie,
		format!("/api/organizations/{id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{fetched:?}");
	assert_eq!(fetched["data"]["type"], "pharmaceutical_company");

	let id = Uuid::parse_str(id)?;
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let rows = sqlx::query_as::<_, (String, serde_json::Value)>("SELECT action, new_values FROM audit_logs WHERE table_name = 'organizations' AND record_id = $1 AND action = 'UPDATE' ORDER BY id").bind(id).fetch_all(&mut *tx).await?;
	tx.rollback().await?;
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].0, "UPDATE");
	assert_eq!(rows[0].1["org_type"], "pharmaceutical_company");
	Ok(())
}
