use crate::authorization_test_support::{
	apply_authorization_isolation_migration, init_authorization_test_db,
	init_clean_bootstrap_authorization_test_db,
};
use crate::common::Result;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn role_label_cannot_enable_platform_rls_bypass() -> Result<()> {
	let database = init_authorization_test_db().await?;
	let organization_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;
	let actor_id = Uuid::new_v4();
	let custom_role_id = Uuid::new_v4();
	sqlx::query("INSERT INTO users (id, role) VALUES ($1, $2)")
		.bind(actor_id)
		.bind(custom_role_id.to_string())
		.execute(database.pool())
		.await?;
	sqlx::query(
		"INSERT INTO user_organization_memberships (user_id, organization_id) VALUES ($1, $2)",
	)
	.bind(actor_id)
	.bind(organization_id)
	.execute(database.pool())
	.await?;
	sqlx::query(
		"INSERT INTO authorization_roles (id, organization_id, role_class, name, built_in) VALUES ($1, $2, 'custom', 'Custom role', false)",
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
	apply_authorization_isolation_migration(&database).await?;

	let runtime = database.runtime_pool().await?;
	let mut transaction = runtime.begin().await?;
	sqlx::query("SELECT set_current_user_context($1)")
		.bind(actor_id)
		.execute(&mut *transaction)
		.await?;
	let result = sqlx::query("SELECT set_org_context($1, 'system_admin'::varchar)")
		.bind(organization_id)
		.execute(&mut *transaction)
		.await;

	assert!(
		result.is_err(),
		"a caller-controlled role label must not create platform isolation bypass"
	);
	transaction.rollback().await?;
	runtime.close().await;
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_rls_does_not_recompute_permissions_from_a_role_label() -> Result<()> {
	let database = init_clean_bootstrap_authorization_test_db().await?;
	apply_authorization_isolation_migration(&database).await?;
	let policy_expression = sqlx::query_scalar::<_, String>(
		"SELECT qual FROM pg_policies WHERE schemaname = current_schema() AND tablename = 'audit_logs' AND policyname = 'audit_logs_read_for_admin_manager'",
	)
	.fetch_one(database.pool())
	.await?;

	assert!(
		!policy_expression.contains("current_user_role")
			&& !policy_expression.contains("permission_profiles"),
		"audit RLS must enforce isolation only, not duplicate application authorization: {policy_expression}"
	);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn clean_bootstrap_has_no_role_label_based_rls_policy() -> Result<()> {
	let database = init_clean_bootstrap_authorization_test_db().await?;
	let role_dependent_policies = sqlx::query_scalar::<_, i64>(
		"SELECT count(*) FROM pg_policies WHERE schemaname = current_schema() AND COALESCE(qual, '') LIKE '%current_user_role%'",
	)
	.fetch_one(database.pool())
	.await?;

	assert_eq!(role_dependent_policies, 0);
	let system_assignment = sqlx::query_scalar::<_, i64>(
		"SELECT count(*) FROM user_role_assignments WHERE user_id = '00000000-0000-0000-0000-000000000001'::uuid AND organization_id = '00000000-0000-0000-0000-000000000000'::uuid AND role_id = '00000000-0000-0000-0000-000000000101'::uuid AND active",
	)
	.fetch_one(database.pool())
	.await?;
	assert_eq!(system_assignment, 1);
	database.close().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fixed_platform_assignment_enables_transaction_local_bypass_only(
) -> Result<()> {
	let database = init_authorization_test_db().await?;
	apply_authorization_isolation_migration(&database).await?;
	let runtime = database.runtime_pool().await?;
	let platform_user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000011")?;
	let organization_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;
	let mut transaction = runtime.begin().await?;
	sqlx::query("SELECT set_current_user_context($1)")
		.bind(platform_user_id)
		.execute(&mut *transaction)
		.await?;
	sqlx::query("SELECT set_authorization_isolation_context($1, true)")
		.bind(organization_id)
		.execute(&mut *transaction)
		.await?;
	assert!(
		sqlx::query_scalar::<_, bool>("SELECT is_current_user_admin()")
			.fetch_one(&mut *transaction)
			.await?
	);
	transaction.rollback().await?;

	let mut reused = runtime.begin().await?;
	assert!(
		!sqlx::query_scalar::<_, bool>("SELECT is_current_user_admin()")
			.fetch_one(&mut *reused)
			.await?
	);
	reused.rollback().await?;
	runtime.close().await;
	database.close().await?;
	Ok(())
}
