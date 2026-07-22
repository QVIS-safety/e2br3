use crate::authorization_test_support::{
	apply_authorization_isolation_migration, apply_authorization_revision_migration,
	init_authorization_test_db,
};
use crate::common::Result;
use lib_core::authorization::{policy_registry, ActionId, DenialReason};
use lib_core::model::authorization::{
	AuthorizationAuditEvent, AuthorizationAuditRepository, SnapshotRepository,
};
use serial_test::serial;
use time::OffsetDateTime;
use uuid::Uuid;

const USER_ID: Uuid = Uuid::from_u128(0x11);
const ORGANIZATION_ID: Uuid = Uuid::from_u128(0x1);

#[serial]
#[tokio::test]
async fn allowed_event_rolls_back_with_mutation_but_denial_survives() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	apply_authorization_isolation_migration(&database).await?;
	let snapshot = SnapshotRepository::load_repeatable_read(
		database.pool(),
		policy_registry(),
		USER_ID,
		ORGANIZATION_ID,
		OffsetDateTime::now_utc(),
		None,
	)
	.await?;
	let action = ActionId::parse("case.update")?;
	let request_id = Uuid::new_v4();

	let allowed = AuthorizationAuditEvent::allowed(
		&snapshot,
		action.clone(),
		request_id,
		Some("case:00000000-0000-0000-0000-000000000901".into()),
	);
	let mut transaction = database.pool().begin().await?;
	AuthorizationAuditRepository::append_allowed(&mut transaction, &allowed).await?;
	transaction.rollback().await?;
	let count: i64 =
		sqlx::query_scalar("SELECT count(*) FROM authorization_audit_events")
			.fetch_one(database.pool())
			.await?;
	assert_eq!(count, 0);
	assert!(AuthorizationAuditRepository::append_denial_after_rollback(
		database.pool(),
		&allowed,
	)
	.await
	.is_err());

	let denied = AuthorizationAuditEvent::denied(
		&snapshot,
		action,
		request_id,
		Some("case:00000000-0000-0000-0000-000000000901".into()),
		DenialReason::MissingEntitlement,
	);
	let mut wrong_transaction = database.pool().begin().await?;
	assert!(AuthorizationAuditRepository::append_allowed(
		&mut wrong_transaction,
		&denied,
	)
	.await
	.is_err());
	wrong_transaction.rollback().await?;
	AuthorizationAuditRepository::append_denial_after_rollback(
		database.pool(),
		&denied,
	)
	.await?;
	let row: (String, Option<String>) = sqlx::query_as(
		"SELECT decision, denial_reason FROM authorization_audit_events",
	)
	.fetch_one(database.pool())
	.await?;
	assert_eq!(row, ("denied".into(), Some("missing_entitlement".into())));
	let update_error = sqlx::query(
		"UPDATE authorization_audit_events SET action_id = 'case.delete'",
	)
	.execute(database.pool())
	.await
	.unwrap_err();
	assert_eq!(
		update_error
			.as_database_error()
			.and_then(|error| error.code()),
		Some(std::borrow::Cow::Borrowed("55000"))
	);
	let delete_error = sqlx::query("DELETE FROM authorization_audit_events")
		.execute(database.pool())
		.await
		.unwrap_err();
	assert_eq!(
		delete_error
			.as_database_error()
			.and_then(|error| error.code()),
		Some(std::borrow::Cow::Borrowed("55000"))
	);

	database.close().await?;
	Ok(())
}
