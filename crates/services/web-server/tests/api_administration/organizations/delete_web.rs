use crate::administration_support::request_json;
use crate::common::{
	cookie_header, init_test_mm, insert_user, insert_user_organization_membership,
	seed_org_with_all_roles, system_user_id, Result,
};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{ROLE_SYSTEM_ADMIN, ROLE_USER, SYSTEM_ORG_ID};
use lib_core::model::store::set_full_context_dbx;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn delete_deactivates_and_preserves_tenant_data_with_single_update_audit(
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
		Some(json!({"data": {"name": format!("Deactivate {}", Uuid::new_v4()), "type": "CRO"}})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{created:?}");
	let organization_id = Uuid::parse_str(
		created["data"]["id"]
			.as_str()
			.ok_or("missing organization id")?,
	)?;
	let member =
		insert_user(&mm, organization_id, ROLE_USER, system_user_id(), None).await?;
	let member_cookie = cookie_header(
		&generate_web_token(&member.email, member.token_salt)?.to_string(),
	);
	insert_user_organization_membership(&mm, admin.id, organization_id).await?;
	let (status, member_profile) =
		request_json(&app, "GET", &member_cookie, "/api/users/me/profile", None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{member_profile:?}");
	assert_eq!(
		member_profile["data"]["activeOrganization"]["id"],
		organization_id.to_string()
	);
	let (status, profile) =
		request_json(&app, "GET", &cookie, "/api/users/me/profile", None).await?;
	assert_eq!(status, StatusCode::OK, "{profile:?}");
	assert!(
		profile["data"]["availableOrganizations"]
			.as_array()
			.ok_or("missing availableOrganizations")?
			.iter()
			.any(|organization| organization["id"] == organization_id.to_string()),
		"{profile:?}"
	);

	for _ in 0..2 {
		let (status, body) = request_json(
			&app,
			"DELETE",
			&cookie,
			format!("/api/organizations/{organization_id}"),
			None,
		)
		.await?;
		assert_eq!(status, StatusCode::NO_CONTENT, "{body:?}");
	}
	let (status, body) = request_json(
		&app,
		"GET",
		&cookie,
		format!("/api/organizations/{organization_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["active"], json!(false));
	let (status, body) =
		request_json(&app, "GET", &member_cookie, "/api/users/me/profile", None)
			.await?;
	assert_eq!(status, StatusCode::UNAUTHORIZED, "{body:?}");
	assert_eq!(body["error"]["message"], "NO_AUTH", "{body:?}");
	let (status, profile) =
		request_json(&app, "GET", &cookie, "/api/users/me/profile", None).await?;
	assert_eq!(status, StatusCode::OK, "{profile:?}");
	assert!(
		profile["data"]["availableOrganizations"]
			.as_array()
			.ok_or("missing availableOrganizations")?
			.iter()
			.all(|organization| organization["id"] != organization_id.to_string()),
		"{profile:?}"
	);

	let system_org_id = Uuid::parse_str(SYSTEM_ORG_ID)?;
	let (status, body) = request_json(
		&app,
		"DELETE",
		&cookie,
		format!("/api/organizations/{system_org_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	let (status, body) = request_json(
		&app,
		"PUT",
		&cookie,
		format!("/api/organizations/{system_org_id}"),
		Some(json!({"data": {"active": false}})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, admin.id, organization_id, ROLE_SYSTEM_ADMIN).await?;
	let (active, settings_exist, user_exists, membership_exists) = dbx
		.fetch_one(
			sqlx::query_as::<_, (bool, bool, bool, bool)>(
				"SELECT
					active,
					EXISTS(SELECT 1 FROM app_settings WHERE organization_id = $1),
					EXISTS(SELECT 1 FROM users WHERE id = $2),
					EXISTS(SELECT 1 FROM user_organization_memberships WHERE organization_id = $1 AND user_id = $2)
				 FROM organizations WHERE id = $1",
			)
			.bind(organization_id)
			.bind(member.id),
		)
		.await?;
	let (system_active,) = dbx
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				"SELECT active FROM organizations WHERE id = $1",
			)
			.bind(system_org_id),
		)
		.await?;
	dbx.rollback_txn().await?;

	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let audits = sqlx::query_as::<_, (String, Value)>(
		"SELECT action, changed_fields FROM audit_logs
		 WHERE table_name = 'organizations' AND record_id = $1
		   AND action IN ('UPDATE', 'DELETE') ORDER BY created_at",
	)
	.bind(organization_id)
	.fetch_all(&mut *tx)
	.await?;
	tx.rollback().await?;

	assert!(!active);
	assert!(settings_exist);
	assert!(user_exists);
	assert!(membership_exists);
	assert!(system_active);
	assert_eq!(audits.len(), 1, "{audits:?}");
	assert_eq!(audits[0].0, "UPDATE");
	assert_eq!(audits[0].1["active"]["old"], json!(true));
	assert_eq!(audits[0].1["active"]["new"], json!(false));
	Ok(())
}
