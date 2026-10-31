use crate::administration_support::request_json;
use crate::common::{
	cookie_header, init_test_mm, insert_user, seed_org_with_all_roles,
	system_user_id, Result,
};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{ROLE_SYSTEM_ADMIN, ROLE_USER};
use lib_core::model::store::set_full_context_dbx;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn workflow_options_are_active_same_org_sorted_limited_dto_rows() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let other = seed_org_with_all_roles(&mm).await?;
	let admin = insert_user(
		&mm,
		seed.org_id,
		ROLE_SYSTEM_ADMIN,
		system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let first =
		insert_user(&mm, seed.org_id, ROLE_USER, system_user_id(), None).await?;
	let inactive =
		insert_user(&mm, seed.org_id, ROLE_USER, system_user_id(), None).await?;
	let outside =
		insert_user(&mm, other.org_id, ROLE_USER, system_user_id(), None).await?;
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, admin.id, seed.org_id, ROLE_SYSTEM_ADMIN).await?;
	let updated = dbx
		.execute(
			sqlx::query("UPDATE users SET active = false WHERE id = $1")
				.bind(inactive.id),
		)
		.await?;
	assert_eq!(updated, 1);
	dbx.commit_txn().await?;
	let cookie = cookie_header(
		&generate_web_token(&admin.email, admin.token_salt)?.to_string(),
	);
	let app = web_server::app(mm);
	let (status, body) = request_json(
		&app,
		"GET",
		&cookie,
		"/api/users/workflow-options?limit=500",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let rows = body["data"]
		.as_array()
		.ok_or("missing workflow options array")?;
	for row in rows {
		assert!(row.get("id").and_then(|value| value.as_str()).is_some());
		assert!(row.get("email").and_then(|value| value.as_str()).is_some());
		assert!(row
			.get("displayName")
			.and_then(|value| value.as_str())
			.is_some());
		assert!(row.get("organizationId").is_none());
	}
	let emails = rows
		.iter()
		.map(|row| row["email"].as_str().ok_or("missing option email"))
		.collect::<std::result::Result<Vec<_>, _>>()?;
	let mut expected = vec![
		seed.admin.email.as_str(),
		seed.manager.email.as_str(),
		seed.user.email.as_str(),
		seed.viewer.email.as_str(),
		admin.email.as_str(),
		first.email.as_str(),
	];
	expected.sort_by_key(|email| email.to_ascii_lowercase());
	assert_eq!(emails, expected);
	assert!(!emails.contains(&inactive.email.as_str()));
	assert!(!emails.contains(&outside.email.as_str()));
	let (status, limited) = request_json(
		&app,
		"GET",
		&cookie,
		"/api/users/workflow-options?limit=1",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{limited:?}");
	assert_eq!(limited["data"].as_array().map(Vec::len), Some(1));
	assert_eq!(limited["data"][0], rows[0]);
	Ok(())
}
