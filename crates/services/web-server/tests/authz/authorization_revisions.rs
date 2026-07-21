use crate::authorization_test_support::{
	apply_authorization_revision_migration, init_authorization_test_db,
	init_clean_bootstrap_authorization_test_db, scalar_i64, AuthorizationTestDb,
};
use crate::common::Result;
use lib_core::authorization::policy_registry;
use lib_core::model::authorization::{
	AuthorizationMigrationService, RevisionRepository,
};
use serial_test::serial;
use uuid::Uuid;

const USER_ID: Uuid = Uuid::from_u128(0x11);
const ORGANIZATION_ID: Uuid = Uuid::from_u128(0x1);

async fn revisions(database: &AuthorizationTestDb) -> Result<(i64, i64)> {
	let revision =
		RevisionRepository::load(database.pool(), USER_ID, ORGANIZATION_ID).await?;
	Ok((revision.organization, revision.principal))
}

#[serial]
#[tokio::test]
async fn registered_fact_triggers_are_complete() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	RevisionRepository::verify_fact_triggers(database.pool(), policy_registry())
		.await?;
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn disabled_fact_trigger_fails_readiness_verification() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::query("ALTER TABLE users DISABLE TRIGGER authz_revision_principal_users")
		.execute(database.pool())
		.await?;
	let error =
		RevisionRepository::verify_fact_triggers(database.pool(), policy_registry())
			.await
			.unwrap_err();
	assert!(matches!(error, sqlx::Error::Protocol(_)));
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn clean_bootstrap_installs_the_same_registered_revision_triggers(
) -> Result<()> {
	let database = init_clean_bootstrap_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	RevisionRepository::verify_fact_triggers(database.pool(), policy_registry())
		.await?;
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn new_organizations_and_memberships_receive_revision_rows_atomically(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO organizations (id, name, org_type) VALUES (
			'00000000-0000-0000-0000-000000000361', 'New organization', 'cro'
		);
		INSERT INTO users (id, role) VALUES (
			'00000000-0000-0000-0000-000000000362', 'user'
		);
		INSERT INTO user_organization_memberships (user_id, organization_id) VALUES (
			'00000000-0000-0000-0000-000000000362',
			'00000000-0000-0000-0000-000000000361'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	let created = RevisionRepository::load(
		database.pool(),
		Uuid::from_u128(0x362),
		Uuid::from_u128(0x361),
	)
	.await?;
	assert_eq!((created.organization, created.principal), (1, 1));
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn principal_fact_changes_advance_only_principal_revision() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	for statement in [
		"UPDATE users SET active = false WHERE id = '00000000-0000-0000-0000-000000000011'",
		"UPDATE users SET access_start_at = now() WHERE id = '00000000-0000-0000-0000-000000000011'",
		"UPDATE users SET access_sender_ids = 'sender-a' WHERE id = '00000000-0000-0000-0000-000000000011'",
		"UPDATE users SET access_blind_allowed = true WHERE id = '00000000-0000-0000-0000-000000000011'",
		"UPDATE users SET active_sender_identifier = 'sender-a' WHERE id = '00000000-0000-0000-0000-000000000011'",
		"UPDATE user_organization_memberships SET active = false WHERE user_id = '00000000-0000-0000-0000-000000000011' AND organization_id = '00000000-0000-0000-0000-000000000001'",
		"UPDATE user_role_assignments SET active = false WHERE user_id = '00000000-0000-0000-0000-000000000011' AND organization_id = '00000000-0000-0000-0000-000000000001'",
	] {
		let before = revisions(&database).await?;
		sqlx::query(statement).execute(database.pool()).await?;
		let after = revisions(&database).await?;
		assert_eq!(after.0, before.0, "organization revision changed for {statement}");
		assert_eq!(after.1, before.1 + 1, "principal revision did not advance once for {statement}");
	}
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn organization_fact_changes_advance_only_organization_revision() -> Result<()>
{
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO permission_profiles (id, organization_id, name)
		VALUES ('00000000-0000-0000-0000-000000000351', '00000000-0000-0000-0000-000000000001', 'Revision role');
		INSERT INTO sender_presaves (id, organization_id, deleted)
		VALUES ('00000000-0000-0000-0000-000000000352', '00000000-0000-0000-0000-000000000001', false);
		INSERT INTO product_presaves (id, organization_id, deleted)
		VALUES ('00000000-0000-0000-0000-000000000353', '00000000-0000-0000-0000-000000000001', false);
		INSERT INTO study_presaves (id, organization_id, deleted)
		VALUES ('00000000-0000-0000-0000-000000000354', '00000000-0000-0000-0000-000000000001', false);
		"#,
	)
	.execute(database.pool())
	.await?;
	AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await?;

	for statement in [
		"UPDATE organizations SET active = false WHERE id = '00000000-0000-0000-0000-000000000001'",
		"UPDATE organizations SET org_type = 'pharmaceutical_company' WHERE id = '00000000-0000-0000-0000-000000000001'",
		"UPDATE authorization_roles SET active = false WHERE id = '00000000-0000-0000-0000-000000000351'",
		"UPDATE authorization_roles SET active = true WHERE id = '00000000-0000-0000-0000-000000000351'",
		"INSERT INTO role_grants (role_id, grant_id) VALUES ('00000000-0000-0000-0000-000000000351', 'case.read')",
		"UPDATE sender_presaves SET deleted = true WHERE id = '00000000-0000-0000-0000-000000000352'",
		"UPDATE product_presaves SET deleted = true WHERE id = '00000000-0000-0000-0000-000000000353'",
		"UPDATE study_presaves SET deleted = true WHERE id = '00000000-0000-0000-0000-000000000354'",
	] {
		let before = revisions(&database).await?;
		sqlx::query(statement).execute(database.pool()).await?;
		let after = revisions(&database).await?;
		assert_eq!(after.0, before.0 + 1, "organization revision did not advance once for {statement}");
		assert_eq!(after.1, before.1, "principal revision changed for {statement}");
	}
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn organization_type_cannot_strand_a_sponsor_admin_assignment() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::query(
		"UPDATE user_role_assignments SET role_id = '00000000-0000-0000-0000-000000000102' WHERE user_id = '00000000-0000-0000-0000-000000000011' AND organization_id = '00000000-0000-0000-0000-000000000001'",
	)
	.execute(database.pool())
	.await?;
	let before = revisions(&database).await?;
	let error = sqlx::query(
		"UPDATE organizations SET org_type = 'pharmaceutical_company' WHERE id = '00000000-0000-0000-0000-000000000001'",
	)
	.execute(database.pool())
	.await
	.unwrap_err();
	assert_eq!(
		error
			.as_database_error()
			.and_then(|error| error.code())
			.as_deref(),
		Some("23514")
	);
	assert_eq!(revisions(&database).await?, before);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn organization_type_and_sponsor_assignment_changes_share_one_lock(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::query(
		"DELETE FROM user_role_assignments WHERE user_id = '00000000-0000-0000-0000-000000000011' AND organization_id = '00000000-0000-0000-0000-000000000001'",
	)
	.execute(database.pool())
	.await?;

	let mut organization_change = database.pool().begin().await?;
	sqlx::query(
		"UPDATE organizations SET org_type = 'pharmaceutical_company' WHERE id = '00000000-0000-0000-0000-000000000001'",
	)
	.execute(&mut *organization_change)
	.await?;
	let assignment_pool = database.pool().clone();
	let (assignment_started, assignment_ready) = tokio::sync::oneshot::channel();
	let mut assignment = tokio::spawn(async move {
		let _ = assignment_started.send(());
		sqlx::query(
			"INSERT INTO user_role_assignments (user_id, organization_id, role_id) VALUES ('00000000-0000-0000-0000-000000000011', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000102')",
		)
		.execute(&assignment_pool)
		.await
	});
	assignment_ready.await.expect("assignment task started");
	assert!(tokio::time::timeout(
		std::time::Duration::from_millis(250),
		&mut assignment
	)
	.await
	.is_err());
	organization_change.commit().await?;
	let error = assignment
		.await
		.expect("assignment task joined")
		.unwrap_err();
	assert_eq!(
		error
			.as_database_error()
			.and_then(|error| error.code())
			.as_deref(),
		Some("23514")
	);

	sqlx::query(
		"UPDATE organizations SET org_type = 'cro' WHERE id = '00000000-0000-0000-0000-000000000001'",
	)
	.execute(database.pool())
	.await?;
	let mut assignment_change = database.pool().begin().await?;
	sqlx::query(
		"INSERT INTO user_role_assignments (user_id, organization_id, role_id) VALUES ('00000000-0000-0000-0000-000000000011', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000102')",
	)
	.execute(&mut *assignment_change)
	.await?;
	let organization_pool = database.pool().clone();
	let (organization_started, organization_ready) = tokio::sync::oneshot::channel();
	let mut organization = tokio::spawn(async move {
		let _ = organization_started.send(());
		sqlx::query(
			"UPDATE organizations SET org_type = 'pharmaceutical_company' WHERE id = '00000000-0000-0000-0000-000000000001'",
		)
		.execute(&organization_pool)
		.await
	});
	organization_ready.await.expect("organization task started");
	assert!(tokio::time::timeout(
		std::time::Duration::from_millis(250),
		&mut organization
	)
	.await
	.is_err());
	assignment_change.commit().await?;
	let error = organization
		.await
		.expect("organization task joined")
		.unwrap_err();
	assert_eq!(
		error
			.as_database_error()
			.and_then(|error| error.code())
			.as_deref(),
		Some("23514")
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ownership_moves_invalidate_both_old_and_new_domains() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO organizations (id, name, org_type) VALUES (
			'00000000-0000-0000-0000-000000000371', 'Second organization', 'cro'
		);
		INSERT INTO user_organization_memberships (user_id, organization_id) VALUES (
			'00000000-0000-0000-0000-000000000011',
			'00000000-0000-0000-0000-000000000371'
		);
		INSERT INTO authorization_roles (id, organization_id, role_class, name, built_in) VALUES
			('00000000-0000-0000-0000-000000000372', '00000000-0000-0000-0000-000000000001', 'custom', 'Movable A', false),
			('00000000-0000-0000-0000-000000000373', '00000000-0000-0000-0000-000000000371', 'custom', 'Movable B', false);
		INSERT INTO role_grants (role_id, grant_id) VALUES (
			'00000000-0000-0000-0000-000000000372', 'case.read'
		);
		"#,
	)
	.execute(database.pool())
	.await?;

	let org_one_before = scalar_i64(
		&database,
		"SELECT revision FROM organization_policy_state WHERE organization_id = '00000000-0000-0000-0000-000000000001'",
	)
	.await?;
	let org_two_before = scalar_i64(
		&database,
		"SELECT revision FROM organization_policy_state WHERE organization_id = '00000000-0000-0000-0000-000000000371'",
	)
	.await?;
	sqlx::query(
		"UPDATE role_grants SET role_id = '00000000-0000-0000-0000-000000000373' WHERE role_id = '00000000-0000-0000-0000-000000000372' AND grant_id = 'case.read'",
	)
	.execute(database.pool())
	.await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT revision FROM organization_policy_state WHERE organization_id = '00000000-0000-0000-0000-000000000001'"
		)
		.await?,
		org_one_before + 1
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT revision FROM organization_policy_state WHERE organization_id = '00000000-0000-0000-0000-000000000371'"
		)
		.await?,
		org_two_before + 1
	);
	let org_one_before = scalar_i64(
		&database,
		"SELECT revision FROM organization_policy_state WHERE organization_id = '00000000-0000-0000-0000-000000000001'",
	)
	.await?;
	let org_two_before = scalar_i64(
		&database,
		"SELECT revision FROM organization_policy_state WHERE organization_id = '00000000-0000-0000-0000-000000000371'",
	)
	.await?;
	sqlx::query(
		"UPDATE authorization_roles SET organization_id = '00000000-0000-0000-0000-000000000371' WHERE id = '00000000-0000-0000-0000-000000000372'",
	)
	.execute(database.pool())
	.await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT revision FROM organization_policy_state WHERE organization_id = '00000000-0000-0000-0000-000000000001'"
		)
		.await?,
		org_one_before + 1
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT revision FROM organization_policy_state WHERE organization_id = '00000000-0000-0000-0000-000000000371'"
		)
		.await?,
		org_two_before + 1
	);

	let old_principal =
		RevisionRepository::load(database.pool(), USER_ID, ORGANIZATION_ID).await?;
	let new_principal =
		RevisionRepository::load(database.pool(), USER_ID, Uuid::from_u128(0x371))
			.await?;
	sqlx::query(
		"UPDATE user_role_assignments SET organization_id = '00000000-0000-0000-0000-000000000371' WHERE user_id = '00000000-0000-0000-0000-000000000011' AND organization_id = '00000000-0000-0000-0000-000000000001'",
	)
	.execute(database.pool())
	.await?;
	let old_after =
		RevisionRepository::load(database.pool(), USER_ID, ORGANIZATION_ID).await?;
	let new_after =
		RevisionRepository::load(database.pool(), USER_ID, Uuid::from_u128(0x371))
			.await?;
	assert_eq!(old_after.principal, old_principal.principal + 1);
	assert_eq!(new_after.principal, new_principal.principal + 1);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn unrelated_updates_and_idempotent_reconciliation_do_not_advance_revisions(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	let before = revisions(&database).await?;
	sqlx::query(
		"UPDATE users SET comments = 'not an authorization fact' WHERE id = '00000000-0000-0000-0000-000000000011'",
	)
	.execute(database.pool())
	.await?;
	sqlx::query(
		"UPDATE users SET active = active WHERE id = '00000000-0000-0000-0000-000000000011'",
	)
	.execute(database.pool())
	.await?;
	AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await?;
	assert_eq!(revisions(&database).await?, before);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn missing_revision_state_fails_closed() -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_revision_migration(&database).await?;
	sqlx::query(
		"DELETE FROM principal_authorization_state WHERE user_id = $1 AND organization_id = $2",
	)
	.bind(USER_ID)
	.bind(ORGANIZATION_ID)
	.execute(database.pool())
	.await?;
	let error = RevisionRepository::load(database.pool(), USER_ID, ORGANIZATION_ID)
		.await
		.unwrap_err();
	assert!(matches!(error, sqlx::Error::RowNotFound));
	sqlx::query(
		"INSERT INTO principal_authorization_state (user_id, organization_id) VALUES ($1, $2)",
	)
	.bind(USER_ID)
	.bind(ORGANIZATION_ID)
	.execute(database.pool())
	.await?;
	sqlx::query("DELETE FROM organization_policy_state WHERE organization_id = $1")
		.bind(ORGANIZATION_ID)
		.execute(database.pool())
		.await?;
	let error = RevisionRepository::load(database.pool(), USER_ID, ORGANIZATION_ID)
		.await
		.unwrap_err();
	assert!(matches!(error, sqlx::Error::RowNotFound));
	database.close().await?;
	Ok(())
}
