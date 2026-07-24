use crate::authorization_test_support::{
	init_authorization_test_db, init_clean_bootstrap_authorization_test_db,
	scalar_i64, scalar_string,
};
use crate::common::Result;
use lib_core::authorization::{
	export_contract, policy_registry, AUTHORIZATION_CONTRACT_SCHEMA_VERSION,
};
use lib_core::model::authorization::{
	AuthorizationMigrationError, AuthorizationMigrationService,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn normalized_catalog_matches_the_registry() -> Result<()> {
	let database = init_authorization_test_db().await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_grant_catalog"
		)
		.await?,
		policy_registry().grants().len() as i64
	);
	assert_eq!(
		scalar_string(
			&database,
			"SELECT catalog_hash FROM authorization_catalog_state WHERE singleton"
		)
		.await?,
		export_contract(policy_registry())?.catalog_hash
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT schema_version::bigint FROM authorization_catalog_state WHERE singleton"
		)
		.await?,
		i64::from(AUTHORIZATION_CONTRACT_SCHEMA_VERSION)
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM role_grants rg JOIN authorization_grant_catalog g USING (grant_id) WHERE g.availability = 'reserved'"
		)
		.await?,
		0
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn clean_bootstrap_and_upgrade_use_the_same_registry_catalog() -> Result<()> {
	let clean = init_clean_bootstrap_authorization_test_db().await?;
	let upgraded = init_authorization_test_db().await?;
	let clean_hash = scalar_string(
		&clean,
		"SELECT catalog_hash FROM authorization_catalog_state WHERE singleton",
	)
	.await?;
	let upgraded_hash = scalar_string(
		&upgraded,
		"SELECT catalog_hash FROM authorization_catalog_state WHERE singleton",
	)
	.await?;
	assert_eq!(clean_hash, upgraded_hash);
	assert_eq!(
		scalar_i64(
			&clean,
			"SELECT count(*) FROM authorization_roles WHERE built_in"
		)
		.await?,
		scalar_i64(
			&upgraded,
			"SELECT count(*) FROM authorization_roles WHERE built_in"
		)
		.await?
	);
	clean.close().await?;
	upgraded.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fixed_built_ins_and_enabled_memberships_are_normalized() -> Result<()> {
	let database = init_authorization_test_db().await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_roles WHERE built_in"
		)
		.await?,
		5
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM user_organization_memberships m LEFT JOIN user_role_assignments a USING (user_id, organization_id) WHERE m.active AND a.role_id IS NULL"
		)
		.await?,
		0
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn runtime_custom_role_writer_cannot_mutate_a_built_in_role() -> Result<()> {
	let database = init_authorization_test_db().await?;
	let runtime = database.runtime_pool().await?;
	let built_in_id = policy_registry()
		.built_in_identities()
		.first()
		.ok_or("missing built-in identity")?
		.id;
	let mut transaction = runtime.begin().await?;
	sqlx::query(
		"SELECT set_current_user_context('00000000-0000-0000-0000-000000000011')",
	)
	.execute(&mut *transaction)
	.await?;
	sqlx::query(
		"SELECT set_org_context('00000000-0000-0000-0000-000000000001', 'system_admin')",
	)
	.execute(&mut *transaction)
	.await?;
	let error = sqlx::query(
		"SELECT authz_upsert_custom_role($1, $2, $3, true, ARRAY[]::text[])",
	)
	.bind(built_in_id)
	.bind(uuid::Uuid::parse_str(
		"00000000-0000-0000-0000-000000000001",
	)?)
	.bind("spoofed built-in")
	.execute(&mut *transaction)
	.await
	.expect_err("runtime writer must not mutate a built-in role");
	assert_eq!(
		error.as_database_error().and_then(|error| error.code()),
		Some("23514".into())
	);
	transaction.rollback().await?;
	runtime.close().await;
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn runtime_custom_role_writer_cannot_reuse_a_role_id_across_organizations(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	let runtime = database.runtime_pool().await?;
	let first_org = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;
	let second_org = uuid::Uuid::new_v4();
	let role_id = uuid::Uuid::new_v4();
	sqlx::query(
		"INSERT INTO organizations (id, name, org_type) VALUES ($1, 'Second organization', 'cro')",
	)
	.bind(second_org)
	.execute(database.pool())
	.await?;
	let mut transaction = runtime.begin().await?;
	sqlx::query(
		"SELECT set_current_user_context('00000000-0000-0000-0000-000000000011')",
	)
	.execute(&mut *transaction)
	.await?;
	sqlx::query(
		"SELECT set_org_context('00000000-0000-0000-0000-000000000001', 'system_admin')",
	)
	.execute(&mut *transaction)
	.await?;
	sqlx::query(
		"SELECT authz_upsert_custom_role($1, $2, 'First role', true, ARRAY[]::text[])",
	)
	.bind(role_id)
	.bind(first_org)
	.execute(&mut *transaction)
	.await?;
	let error = sqlx::query(
		"SELECT authz_upsert_custom_role($1, $2, 'Cross-org overwrite', true, ARRAY[]::text[])",
	)
	.bind(role_id)
	.bind(second_org)
	.execute(&mut *transaction)
	.await
	.expect_err("runtime writer must not reuse another organization's role id");
	assert_eq!(
		error.as_database_error().and_then(|error| error.code()),
		Some("23514".into())
	);
	transaction.rollback().await?;
	runtime.close().await;
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn runtime_role_writer_rejects_a_custom_role_as_assignment_authority(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	let runtime = database.runtime_pool().await?;
	let organization_id =
		uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;
	let actor_id = uuid::Uuid::new_v4();
	let target_id = uuid::Uuid::new_v4();
	let custom_role_id = uuid::Uuid::new_v4();
	let operational_role_id = policy_registry()
		.built_in_identities()
		.iter()
		.find(|identity| {
			identity.kind
				== lib_core::authorization::BuiltInIdentityKind::OperationalUser
		})
		.ok_or("missing operational identity")?
		.id;
	sqlx::query("INSERT INTO users (id, role) VALUES ($1, $2), ($3, 'user')")
		.bind(actor_id)
		.bind(custom_role_id.to_string())
		.bind(target_id)
		.execute(database.pool())
		.await?;
	sqlx::query(
		"INSERT INTO user_organization_memberships (user_id, organization_id) VALUES ($1, $3), ($2, $3)",
	)
	.bind(actor_id)
	.bind(target_id)
	.bind(organization_id)
	.execute(database.pool())
	.await?;
	sqlx::query(
		"INSERT INTO authorization_roles (id, organization_id, role_class, name, built_in) VALUES ($1, $2, 'custom', 'Custom administrator', false)",
	)
	.bind(custom_role_id)
	.bind(organization_id)
	.execute(database.pool())
	.await?;
	sqlx::query(
		"INSERT INTO user_role_assignments (user_id, organization_id, role_id) VALUES ($1, $2, $3)",
	)
	.bind(actor_id)
	.bind(organization_id)
	.bind(custom_role_id)
	.execute(database.pool())
	.await?;

	let mut transaction = runtime.begin().await?;
	sqlx::query("SELECT set_current_user_context($1)")
		.bind(actor_id)
		.execute(&mut *transaction)
		.await?;
	sqlx::query("SELECT set_org_context($1, 'custom-role')")
		.bind(organization_id)
		.execute(&mut *transaction)
		.await?;
	let error = sqlx::query("SELECT authz_assign_user_role($1, $2, $3)")
		.bind(target_id)
		.bind(organization_id)
		.bind(operational_role_id)
		.execute(&mut *transaction)
		.await
		.expect_err("a custom role must not become role-assignment authority");
	assert_eq!(
		error.as_database_error().and_then(|error| error.code()),
		Some("42501".into())
	);
	transaction.rollback().await?;
	runtime.close().await;
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn runtime_role_writer_rejects_an_actor_without_an_assignment() -> Result<()> {
	let database = init_authorization_test_db().await?;
	let runtime = database.runtime_pool().await?;
	let organization_id =
		uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;
	let mut transaction = runtime.begin().await?;
	sqlx::query("SELECT set_current_user_context($1)")
		.bind(uuid::Uuid::new_v4())
		.execute(&mut *transaction)
		.await?;
	sqlx::query("SELECT set_org_context($1, 'system_admin')")
		.bind(organization_id)
		.execute(&mut *transaction)
		.await?;
	let error = sqlx::query(
		"SELECT authz_upsert_custom_role($1, $2, 'Unowned role', true, ARRAY[]::text[])",
	)
	.bind(uuid::Uuid::new_v4())
	.bind(organization_id)
	.execute(&mut *transaction)
	.await
	.expect_err("an actor without a normalized assignment must fail closed");
	assert_eq!(
		error.as_database_error().and_then(|error| error.code()),
		Some("42501".into())
	);
	transaction.rollback().await?;
	runtime.close().await;
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn reconciliation_is_idempotent() -> Result<()> {
	let database = init_authorization_test_db().await?;
	let before = scalar_i64(&database, "SELECT count(*) FROM role_grants").await?;
	let assigned_at_before = scalar_string(
		&database,
		"SELECT assigned_at::text FROM user_role_assignments WHERE user_id = '00000000-0000-0000-0000-000000000011'",
	)
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	let after = scalar_i64(&database, "SELECT count(*) FROM role_grants").await?;
	let assigned_at_after = scalar_string(
		&database,
		"SELECT assigned_at::text FROM user_role_assignments WHERE user_id = '00000000-0000-0000-0000-000000000011'",
	)
	.await?;
	assert_eq!(before, after);
	assert_eq!(assigned_at_before, assigned_at_after);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn reconciliation_removes_stale_assignments_and_custom_roles() -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO permission_profiles (id, organization_id, name)
		VALUES (
			'00000000-0000-0000-0000-000000000321',
			'00000000-0000-0000-0000-000000000001',
			'Temporary role'
		);
		INSERT INTO users (id, role) VALUES (
			'00000000-0000-0000-0000-000000000322',
			'00000000-0000-0000-0000-000000000321'
		);
		INSERT INTO user_organization_memberships (user_id, organization_id)
		VALUES (
			'00000000-0000-0000-0000-000000000322',
			'00000000-0000-0000-0000-000000000001'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	sqlx::raw_sql(
		r#"
		DELETE FROM user_organization_memberships
		WHERE user_id = '00000000-0000-0000-0000-000000000322';
		DELETE FROM permission_profiles
		WHERE id = '00000000-0000-0000-0000-000000000321';
		"#,
	)
	.execute(database.pool())
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_roles WHERE id = '00000000-0000-0000-0000-000000000321'"
		)
		.await?,
		0
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_migration_reconciliations WHERE user_id = '00000000-0000-0000-0000-000000000322'"
		)
		.await?,
		0
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn deleted_in_use_profile_is_rejected_without_partial_cleanup() -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO permission_profiles (id, organization_id, name)
		VALUES (
			'00000000-0000-0000-0000-000000000331',
			'00000000-0000-0000-0000-000000000001',
			'Assigned custom role'
		);
		INSERT INTO users (id, role) VALUES (
			'00000000-0000-0000-0000-000000000332',
			'00000000-0000-0000-0000-000000000331'
		);
		INSERT INTO user_organization_memberships (user_id, organization_id)
		VALUES (
			'00000000-0000-0000-0000-000000000332',
			'00000000-0000-0000-0000-000000000001'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	sqlx::query(
		"DELETE FROM permission_profiles WHERE id = '00000000-0000-0000-0000-000000000331'",
	)
	.execute(database.pool())
	.await?;

	let error = AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await
	.unwrap_err();
	assert!(matches!(error, AuthorizationMigrationError::Rejected(_)));
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_migration_rejections WHERE user_id = '00000000-0000-0000-0000-000000000332' AND NOT resolved"
		)
		.await?,
		1
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM user_role_assignments WHERE user_id = '00000000-0000-0000-0000-000000000332' AND role_id = '00000000-0000-0000-0000-000000000331'"
		)
		.await?,
		1
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn custom_role_and_assignment_backfill_use_canonical_grants() -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO permission_profiles (
			id, organization_id, name, privileges_json
		) VALUES (
			'00000000-0000-0000-0000-000000000301',
			'00000000-0000-0000-0000-000000000001',
			'Case Reviewer',
			'[{
				"menu_key":"case", "can_read":true, "can_edit":false,
				"can_review":true, "can_lock":false
			}]'::jsonb
		);
		INSERT INTO users (id, role) VALUES (
			'00000000-0000-0000-0000-000000000302',
			'00000000-0000-0000-0000-000000000301'
		);
		INSERT INTO user_organization_memberships (user_id, organization_id)
		VALUES (
			'00000000-0000-0000-0000-000000000302',
			'00000000-0000-0000-0000-000000000001'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM role_grants WHERE role_id = '00000000-0000-0000-0000-000000000301' AND grant_id IN ('case.read', 'case.review')"
		)
		.await?,
		2
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM user_role_assignments WHERE user_id = '00000000-0000-0000-0000-000000000302' AND role_id = '00000000-0000-0000-0000-000000000301'"
		)
		.await?,
		1
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_migration_reconciliations WHERE user_id = '00000000-0000-0000-0000-000000000302' AND comparison_status = 'pending_action_binding' AND equivalent IS NULL AND jsonb_array_length(legacy_effective_access) > 0 AND jsonb_array_length(normalized_effective_access) > 0"
		)
		.await?,
		1
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn reconciliation_proof_is_preserved_only_for_unchanged_evidence() -> Result<()>
{
	let database = init_authorization_test_db().await?;
	sqlx::query(
		"UPDATE authorization_migration_reconciliations SET comparison_status = 'proven_equivalent', equivalent = true, proof_hash = repeat('a', 64) WHERE user_id = '00000000-0000-0000-0000-000000000011'",
	)
	.execute(database.pool())
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_migration_reconciliations WHERE user_id = '00000000-0000-0000-0000-000000000011' AND comparison_status = 'proven_equivalent' AND equivalent AND proof_hash = repeat('a', 64)"
		)
		.await?,
		1
	);

	sqlx::query(
		"UPDATE users SET role = 'user' WHERE id = '00000000-0000-0000-0000-000000000011'",
	)
	.execute(database.pool())
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_migration_reconciliations WHERE user_id = '00000000-0000-0000-0000-000000000011' AND comparison_status = 'pending_action_binding' AND equivalent IS NULL AND proof_hash IS NULL"
		)
		.await?,
		1
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn same_access_role_identity_change_invalidates_reconciliation_proof(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO permission_profiles (id, organization_id, name) VALUES
			('00000000-0000-0000-0000-000000000341', '00000000-0000-0000-0000-000000000001', 'Empty role A'),
			('00000000-0000-0000-0000-000000000342', '00000000-0000-0000-0000-000000000001', 'Empty role B');
		INSERT INTO users (id, role) VALUES (
			'00000000-0000-0000-0000-000000000343',
			'00000000-0000-0000-0000-000000000341'
		);
		INSERT INTO user_organization_memberships (user_id, organization_id) VALUES (
			'00000000-0000-0000-0000-000000000343',
			'00000000-0000-0000-0000-000000000001'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	sqlx::query(
		"UPDATE authorization_migration_reconciliations SET comparison_status = 'proven_equivalent', equivalent = true, proof_hash = repeat('b', 64) WHERE user_id = '00000000-0000-0000-0000-000000000343'",
	)
	.execute(database.pool())
	.await?;
	sqlx::query(
		"UPDATE users SET role = '00000000-0000-0000-0000-000000000342' WHERE id = '00000000-0000-0000-0000-000000000343'",
	)
	.execute(database.pool())
	.await?;
	crate::authorization_test_support::apply_authorization_migrations(&database)
		.await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_migration_reconciliations WHERE user_id = '00000000-0000-0000-0000-000000000343' AND normalized_role_id = '00000000-0000-0000-0000-000000000342' AND legacy_effective_access = '[]'::jsonb AND normalized_effective_access = '[]'::jsonb AND comparison_status = 'pending_action_binding' AND equivalent IS NULL AND proof_hash IS NULL"
		)
		.await?,
		1
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn reconciliation_status_cannot_contradict_equivalence() -> Result<()> {
	let database = init_authorization_test_db().await?;
	let error = sqlx::query(
		"UPDATE authorization_migration_reconciliations SET comparison_status = 'proven_equivalent', equivalent = false, proof_hash = repeat('a', 64)",
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
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn unknown_active_role_rolls_back_and_persists_rejection() -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO users (id, role) VALUES (
			'00000000-0000-0000-0000-000000000311',
			'not-a-known-role'
		);
		INSERT INTO user_organization_memberships (user_id, organization_id)
		VALUES (
			'00000000-0000-0000-0000-000000000311',
			'00000000-0000-0000-0000-000000000001'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	let error = AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await
	.unwrap_err();
	assert!(matches!(error, AuthorizationMigrationError::Rejected(_)));
	let repeated = AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await
	.unwrap_err();
	assert!(matches!(repeated, AuthorizationMigrationError::Rejected(_)));
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM authorization_migration_rejections WHERE user_id = '00000000-0000-0000-0000-000000000311' AND NOT resolved"
		)
		.await?,
		1
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn reviewed_obsolete_menu_flags_are_dropped_during_one_way_backfill(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO permission_profiles (
			id, organization_id, name, privileges_json
		) VALUES (
			'00000000-0000-0000-0000-000000000391',
			'00000000-0000-0000-0000-000000000001',
			'Legacy cleaned role',
			'[
			  {"menu_key":"case","can_read":true,"can_edit":false,"can_review":false,"can_lock":false},
			  {"menu_key":"users","can_read":true,"can_edit":false,"can_review":false,"can_lock":false},
			  {"menu_key":"data","can_read":true,"can_edit":false,"can_review":false,"can_lock":false},
			  {"menu_key":"audit","can_read":true,"can_edit":false,"can_review":false,"can_lock":false},
			  {"menu_key":"roles","can_read":true,"can_edit":false,"can_review":false,"can_lock":false},
			  {"menu_key":"settings","can_read":true,"can_edit":false,"can_review":false,"can_lock":false},
			  {"menu_key":"home_email","can_read":false,"can_edit":true,"can_review":false,"can_lock":false},
			  {"menu_key":"email_lock","can_read":true,"can_edit":false,"can_review":false,"can_lock":false},
			  {"menu_key":"email_report_due","can_read":false,"can_edit":true,"can_review":false,"can_lock":false}
			]'
		);
		INSERT INTO users (id, role) VALUES (
			'00000000-0000-0000-0000-000000000392',
			'00000000-0000-0000-0000-000000000391'
		);
		INSERT INTO user_organization_memberships (user_id, organization_id)
		VALUES (
			'00000000-0000-0000-0000-000000000392',
			'00000000-0000-0000-0000-000000000001'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await?;
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM role_grants WHERE role_id = '00000000-0000-0000-0000-000000000391' AND grant_id = 'case.read'"
		)
		.await?,
		1
	);
	assert_eq!(
		scalar_i64(
			&database,
			"SELECT count(*) FROM role_grants WHERE role_id = '00000000-0000-0000-0000-000000000391'"
		)
		.await?,
		1
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn genuinely_unknown_menu_flag_still_blocks_backfill() -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::query(
		"INSERT INTO permission_profiles (id, organization_id, name, privileges_json) VALUES ('00000000-0000-0000-0000-000000000393', '00000000-0000-0000-0000-000000000001', 'Unknown role', '[{\"menu_key\":\"not_in_the_reviewed_contract\",\"can_read\":true,\"can_edit\":false,\"can_review\":false,\"can_lock\":false}]')",
	)
	.execute(database.pool())
	.await?;
	let error = AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await
	.unwrap_err();
	assert!(matches!(error, AuthorizationMigrationError::Rejected(_)));
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn database_rejects_reserved_grant_assignment() -> Result<()> {
	let database = init_authorization_test_db().await?;
	let error = sqlx::query(
		"INSERT INTO role_grants (role_id, grant_id) VALUES ('00000000-0000-0000-0000-000000000101', 'email.report_due.read')",
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
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn conflicting_catalog_hash_fails_closed() -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::query(
		"UPDATE authorization_catalog_state SET catalog_hash = repeat('0', 64) WHERE singleton",
	)
	.execute(database.pool())
	.await?;
	let error = AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await
	.unwrap_err();
	assert!(matches!(
		error,
		AuthorizationMigrationError::CatalogHashMismatch { .. }
	));
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn reviewed_catalog_predecessor_upgrades_explicitly() -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::query(
		"UPDATE authorization_catalog_state SET catalog_hash = 'a8cbca03d528beb474ba7beb0658a5e22103df9b1a2693c9cce22708530a79e5' WHERE singleton",
	)
	.execute(database.pool())
	.await?;
	sqlx::raw_sql(include_str!(
		"../../../../../db/migrations/20260722_authorization_ui_binding_catalog.sql"
	))
	.execute(database.pool())
	.await?;
	sqlx::raw_sql(include_str!(
		"../../../../../db/migrations/20260724_authorization_direct_grant_actions.sql"
	))
	.execute(database.pool())
	.await?;
	sqlx::raw_sql(include_str!(
		"../../../../../db/migrations/20260724_authorization_operational_grant_actions.sql"
	))
	.execute(database.pool())
	.await?;
	let stored: String = sqlx::query_scalar(
		"SELECT catalog_hash FROM authorization_catalog_state WHERE singleton",
	)
	.fetch_one(database.pool())
	.await?;
	assert_eq!(
		stored,
		export_contract(policy_registry())?.catalog_hash,
		"only the reviewed predecessor should advance to the deployed catalog"
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn runtime_role_is_read_only_over_normalized_storage() -> Result<()> {
	let database = init_authorization_test_db().await?;
	let runtime = database.runtime_pool().await?;
	let error = sqlx::query(
		"UPDATE authorization_grant_catalog SET availability = 'implemented' WHERE grant_id = 'email.report_due.read'",
	)
	.execute(&runtime)
	.await
	.unwrap_err();
	assert_eq!(
		error
			.as_database_error()
			.and_then(|error| error.code())
			.as_deref(),
		Some("42501")
	);
	let error = sqlx::query(
		"UPDATE user_role_assignments SET role_id = '00000000-0000-0000-0000-000000000101'",
	)
	.execute(&runtime)
	.await
	.unwrap_err();
	assert_eq!(
		error
			.as_database_error()
			.and_then(|error| error.code())
			.as_deref(),
		Some("42501")
	);
	runtime.close().await;
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn assignment_and_role_class_constraints_reject_escalation() -> Result<()> {
	let database = init_authorization_test_db().await?;
	sqlx::raw_sql(
		r#"
		INSERT INTO organizations (id, name, org_type) VALUES (
			'00000000-0000-0000-0000-000000000401',
			'Company organization',
			'pharmaceutical_company'
		);
		INSERT INTO users (id, role) VALUES
			('00000000-0000-0000-0000-000000000402', 'user'),
			('00000000-0000-0000-0000-000000000403', 'user');
		INSERT INTO user_organization_memberships (user_id, organization_id) VALUES
			('00000000-0000-0000-0000-000000000402', '00000000-0000-0000-0000-000000000001'),
			('00000000-0000-0000-0000-000000000403', '00000000-0000-0000-0000-000000000401');
		INSERT INTO authorization_roles (
			id, organization_id, role_class, name, built_in
		) VALUES (
			'00000000-0000-0000-0000-000000000404',
			'00000000-0000-0000-0000-000000000001',
			'custom', 'CRO custom role', false
		);
		"#,
	)
	.execute(database.pool())
	.await?;

	for (user_id, role_id) in [
		(
			"00000000-0000-0000-0000-000000000402",
			"00000000-0000-0000-0000-000000000105",
		),
		(
			"00000000-0000-0000-0000-000000000402",
			"00000000-0000-0000-0000-000000000103",
		),
		(
			"00000000-0000-0000-0000-000000000403",
			"00000000-0000-0000-0000-000000000404",
		),
	] {
		let error = sqlx::query(
			"INSERT INTO user_role_assignments (user_id, organization_id, role_id) SELECT $1::uuid, organization_id, $2::uuid FROM user_organization_memberships WHERE user_id = $1::uuid",
		)
		.bind(user_id)
		.bind(role_id)
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
	}

	let error = sqlx::query(
		"INSERT INTO role_grants (role_id, grant_id) VALUES ('00000000-0000-0000-0000-000000000104', 'admin.edit')",
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
	database.close().await?;
	Ok(())
}
