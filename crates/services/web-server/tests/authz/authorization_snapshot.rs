use crate::authorization_test_support::{
	apply_authorization_revision_migration, init_authorization_test_db,
};
use crate::common::Result;
use axum::extract::FromRequestParts;
use axum::http::Request;
use lib_core::authorization::{policy_registry, BuiltInIdentityKind};
use lib_core::model::authorization::{SnapshotLoadError, SnapshotRepository};
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use serial_test::serial;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

const USER_ID: Uuid = Uuid::from_u128(0x11);
const ORGANIZATION_ID: Uuid = Uuid::from_u128(0x1);

#[serial]
#[tokio::test]
async fn normalized_assignment_builds_one_versioned_platform_snapshot() -> Result<()>
{
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	let evaluated_at = OffsetDateTime::now_utc();
	let snapshot = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		evaluated_at,
		None,
	)
	.await?;

	assert_eq!(snapshot.principal_id(), USER_ID);
	assert_eq!(snapshot.organization_id(), ORGANIZATION_ID);
	assert_eq!(
		snapshot.identity().built_in_kind(),
		Some(BuiltInIdentityKind::PlatformAdministrator)
	);
	assert!(snapshot.entitlements().contains("role.manage"));
	assert!(snapshot.version().organization_revision() > 0);
	assert!(snapshot.version().principal_revision() > 0);
	assert_eq!(snapshot.evaluated_at(), evaluated_at);
	let (mut parts, _) = Request::new(()).into_parts();
	parts
		.extensions
		.insert(AuthorizationSnapshotW::new(snapshot));
	let extracted =
		AuthorizationSnapshotW::from_request_parts(&mut parts, &()).await?;
	assert_eq!(extracted.principal_id(), USER_ID);
	database.close().await?;
	Ok(())
}

#[tokio::test]
async fn protected_snapshot_extractor_fails_when_middleware_did_not_attach_one() {
	let (mut parts, _) = Request::new(()).into_parts();
	let result = AuthorizationSnapshotW::from_request_parts(&mut parts, &()).await;
	assert!(result.is_err());
}

#[serial]
#[tokio::test]
async fn custom_role_name_cannot_spoof_a_built_in_identity() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO permission_profiles (
			id, organization_id, name, privileges_json
		) VALUES (
			'00000000-0000-0000-0000-000000000381',
			'00000000-0000-0000-0000-000000000001',
			'system_admin',
			'[{"menu_key":"admin","can_read":true,"can_edit":true,"can_review":false,"can_lock":false}]'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	lib_core::model::authorization::AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await?;
	sqlx::query(
		"UPDATE user_role_assignments SET role_id = '00000000-0000-0000-0000-000000000381' WHERE user_id = $1 AND organization_id = $2",
	)
	.bind(USER_ID)
	.bind(ORGANIZATION_ID)
	.execute(database.pool())
	.await?;

	let snapshot = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		OffsetDateTime::now_utc(),
		None,
	)
	.await?;
	assert_eq!(snapshot.identity().built_in_kind(), None);
	assert_eq!(snapshot.role_id(), Uuid::from_u128(0x381));
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn principal_scopes_are_request_local_even_for_the_same_role() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	let second_user = Uuid::from_u128(0x382);
	sqlx::query(
		"UPDATE users SET access_sender_ids = '[\"sender-a\"]' WHERE id = $1",
	)
	.bind(USER_ID)
	.execute(database.pool())
	.await?;
	sqlx::query(
		"INSERT INTO users (id, role, access_sender_ids) VALUES ($1, 'system_admin', '[\"sender-b\"]')",
	)
	.bind(second_user)
	.execute(database.pool())
	.await?;
	sqlx::query(
		"INSERT INTO user_organization_memberships (user_id, organization_id) VALUES ($1, $2)",
	)
	.bind(second_user)
	.bind(ORGANIZATION_ID)
	.execute(database.pool())
	.await?;
	lib_core::model::authorization::AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await?;

	let now = OffsetDateTime::now_utc();
	let first = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		now,
		None,
	)
	.await?;
	let second = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		second_user,
		ORGANIZATION_ID,
		now,
		None,
	)
	.await?;
	assert_eq!(first.scope().sender_ids(), &["sender-a"]);
	assert_eq!(second.scope().sender_ids(), &["sender-b"]);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn access_end_is_exclusive_and_missing_revision_rows_fail_closed() -> Result<()>
{
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	let boundary = OffsetDateTime::now_utc() + Duration::hours(1);
	sqlx::query("UPDATE users SET access_end_at = $1 WHERE id = $2")
		.bind(boundary)
		.bind(USER_ID)
		.execute(database.pool())
		.await?;

	let before = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		boundary - Duration::nanoseconds(1),
		None,
	)
	.await?;
	assert_eq!(before.authorization_valid_until(), Some(boundary));
	let at_boundary = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		boundary,
		None,
	)
	.await
	.unwrap_err();
	assert!(matches!(at_boundary, SnapshotLoadError::InactivePrincipal));

	sqlx::query(
		"DELETE FROM principal_authorization_state WHERE user_id = $1 AND organization_id = $2",
	)
	.bind(USER_ID)
	.bind(ORGANIZATION_ID)
	.execute(database.pool())
	.await?;
	let missing = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		boundary - Duration::minutes(1),
		None,
	)
	.await
	.unwrap_err();
	assert!(matches!(missing, SnapshotLoadError::MissingRevisionState));
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn access_start_is_inclusive_and_token_expiry_is_the_earliest_boundary(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	let start = OffsetDateTime::now_utc() + Duration::hours(1);
	let access_end = start + Duration::hours(2);
	let token_expiry = start + Duration::minutes(30);
	sqlx::query(
		"UPDATE users SET access_start_at = $1, access_end_at = $2 WHERE id = $3",
	)
	.bind(start)
	.bind(access_end)
	.bind(USER_ID)
	.execute(database.pool())
	.await?;

	let before = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		start - Duration::nanoseconds(1),
		Some(token_expiry),
	)
	.await
	.unwrap_err();
	assert!(matches!(before, SnapshotLoadError::InactivePrincipal));

	let at_start = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		start,
		Some(token_expiry),
	)
	.await?;
	assert_eq!(at_start.authorization_valid_until(), Some(token_expiry));

	let at_token_expiry = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		token_expiry,
		Some(token_expiry),
	)
	.await
	.unwrap_err();
	assert!(matches!(
		at_token_expiry,
		SnapshotLoadError::InactivePrincipal
	));
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn missing_normalized_assignment_never_falls_back_to_legacy_role() -> Result<()>
{
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::query(
		"DELETE FROM user_role_assignments WHERE user_id = $1 AND organization_id = $2",
	)
	.bind(USER_ID)
	.bind(ORGANIZATION_ID)
	.execute(database.pool())
	.await?;

	let error = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		OffsetDateTime::now_utc(),
		None,
	)
	.await
	.unwrap_err();
	assert!(matches!(
		error,
		SnapshotLoadError::MissingPrincipalAssignment
	));
	database.close().await?;
	Ok(())
}
