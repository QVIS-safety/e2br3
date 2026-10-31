use crate::administration_support::request_json;
use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	system_user_id, Result,
};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN, ROLE_USER};
use lib_core::model::store::set_full_context_dbx;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn delete_hard_deletes_and_audits_or_rolls_back_on_foreign_key_dependency(
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

	let (status, created) = request_json(
		&app,
		"POST",
		&cookie,
		"/api/organizations",
		Some(
			json!({"data": {"name": format!("Delete {}", Uuid::new_v4()), "type": "CRO"}}),
		),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{created:?}");
	let deleted_id = Uuid::parse_str(
		created["data"]["id"]
			.as_str()
			.ok_or("missing organization id")?,
	)?;
	let (status, body) = request_json(
		&app,
		"DELETE",
		&cookie,
		format!("/api/organizations/{deleted_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::NO_CONTENT, "{body:?}");
	let (status, body) = request_json(
		&app,
		"GET",
		&cookie,
		format!("/api/organizations/{deleted_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body:?}");

	let dependent = seed_org_with_all_roles(&mm).await?;
	let dependent_user =
		insert_user(&mm, dependent.org_id, ROLE_USER, system_user_id(), None)
			.await?;
	let (status, body) = request_json(
		&app,
		"DELETE",
		&cookie,
		format!("/api/organizations/{}", dependent.org_id),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, admin.id, seed.org_id, ROLE_SYSTEM_ADMIN).await?;
	let (deleted_exists,) = dbx
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				"SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)",
			)
			.bind(deleted_id),
		)
		.await?;
	let (dependent_exists,) = dbx
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				"SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)",
			)
			.bind(dependent.org_id),
		)
		.await?;
	set_full_context_dbx(
		dbx,
		dependent.admin.id,
		dependent.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (user_exists,) = dbx
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				"SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)",
			)
			.bind(dependent_user.id),
		)
		.await?;
	dbx.rollback_txn().await?;

	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let delete_audits = sqlx::query_as::<_, (Uuid, String)>("SELECT record_id, action FROM audit_logs WHERE table_name = 'organizations' AND record_id = ANY($1) AND action = 'DELETE'").bind(vec![deleted_id, dependent.org_id]).fetch_all(&mut *tx).await?;
	tx.rollback().await?;
	assert!(!deleted_exists);
	assert!(dependent_exists);
	assert!(user_exists);
	assert_eq!(delete_audits, vec![(deleted_id, "DELETE".to_string())]);
	Ok(())
}
