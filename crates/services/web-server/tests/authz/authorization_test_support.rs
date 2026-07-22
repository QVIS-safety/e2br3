use crate::common::{init_test_env, Result};
use lib_core::authorization::policy_registry;
use lib_core::model::authorization::AuthorizationMigrationService;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

pub struct AuthorizationTestDb {
	pool: Pool<Postgres>,
	schema: String,
	database_url: String,
}

impl AuthorizationTestDb {
	pub fn pool(&self) -> &Pool<Postgres> {
		&self.pool
	}

	pub async fn runtime_pool(&self) -> Result<Pool<Postgres>> {
		let connect_schema = self.schema.clone();
		Ok(PgPoolOptions::new()
			.max_connections(1)
			.after_connect(move |connection, _| {
				let statement = format!("SET search_path TO \"{connect_schema}\"");
				Box::pin(async move {
					sqlx::query(&statement).execute(&mut *connection).await?;
					sqlx::query("SET ROLE e2br3_app_role")
						.execute(&mut *connection)
						.await?;
					Ok(())
				})
			})
			.connect(&self.database_url)
			.await?)
	}

	pub async fn close(self) -> Result<()> {
		self.pool.close().await;
		let admin = PgPoolOptions::new()
			.max_connections(1)
			.connect(&self.database_url)
			.await?;
		sqlx::query(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
			.execute(&admin)
			.await?;
		admin.close().await;
		Ok(())
	}
}

pub async fn init_authorization_test_db() -> Result<AuthorizationTestDb> {
	let database = new_isolated_database().await?;

	sqlx::raw_sql(
		r#"
		CREATE FUNCTION set_current_user_context(target_user_id uuid)
		RETURNS void LANGUAGE sql AS $$
			SELECT set_config('app.current_user_id', target_user_id::text, true)
		$$;
		CREATE FUNCTION set_org_context(target_organization_id uuid, target_role text)
		RETURNS void LANGUAGE sql AS $$
			SELECT set_config('app.current_organization_id', target_organization_id::text, true);
			SELECT set_config('app.current_user_role', target_role, true)
		$$;
		CREATE FUNCTION current_organization_id() RETURNS uuid
		LANGUAGE sql STABLE AS $$
			SELECT NULLIF(current_setting('app.current_organization_id', true), '')::uuid
		$$;
		CREATE TABLE organizations (
			id uuid PRIMARY KEY,
			name text NOT NULL,
			org_type text,
			active boolean DEFAULT true
		);
		CREATE TABLE users (
			id uuid PRIMARY KEY,
			role text NOT NULL,
			active boolean DEFAULT true,
			access_start_at timestamptz,
			access_end_at timestamptz,
			access_sender_ids text,
			access_product_ids text,
			access_study_ids text,
			access_blind_allowed boolean,
			active_sender_identifier text,
			comments text
		);
		CREATE TABLE user_organization_memberships (
			user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			organization_id uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
			active boolean NOT NULL DEFAULT true,
			PRIMARY KEY (user_id, organization_id)
		);
		CREATE TABLE permission_profiles (
			id uuid PRIMARY KEY,
			organization_id uuid NOT NULL REFERENCES organizations(id),
			name text NOT NULL,
			built_in boolean NOT NULL DEFAULT false,
			active boolean NOT NULL DEFAULT true,
			privileges_json jsonb NOT NULL DEFAULT '[]'::jsonb
		);
		CREATE TABLE sender_presaves (
			id uuid PRIMARY KEY,
			organization_id uuid NOT NULL REFERENCES organizations(id),
			deleted boolean NOT NULL DEFAULT false
		);
		CREATE TABLE product_presaves (
			id uuid PRIMARY KEY,
			organization_id uuid NOT NULL REFERENCES organizations(id),
			deleted boolean NOT NULL DEFAULT false
		);
		CREATE TABLE study_presaves (
			id uuid PRIMARY KEY,
			organization_id uuid NOT NULL REFERENCES organizations(id),
			deleted boolean NOT NULL DEFAULT false
		);
		CREATE TABLE audit_logs (
			id bigserial PRIMARY KEY,
			organization_id uuid NOT NULL
		);
		ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;
		CREATE POLICY audit_logs_read_for_admin_manager ON audit_logs
			FOR SELECT TO e2br3_app_role USING (true);
		INSERT INTO organizations (id, name, org_type)
		VALUES (
			'00000000-0000-0000-0000-000000000001',
			'Test organization',
			'cro'
		);
		INSERT INTO users (id, role)
		VALUES ('00000000-0000-0000-0000-000000000011', 'system_admin');
		INSERT INTO user_organization_memberships (user_id, organization_id)
		VALUES (
			'00000000-0000-0000-0000-000000000011',
			'00000000-0000-0000-0000-000000000001'
		);
		"#,
	)
	.execute(database.pool())
	.await?;
	apply_authorization_migrations(&database).await?;
	Ok(database)
}

pub async fn init_clean_bootstrap_authorization_test_db(
) -> Result<AuthorizationTestDb> {
	let database = new_isolated_database().await?;
	sqlx::raw_sql(include_str!(
		"../../../../../db/bootstrap/01-safetydb-schema.sql"
	))
	.execute(database.pool())
	.await?;
	apply_authorization_migrations(&database).await?;
	Ok(database)
}

async fn new_isolated_database() -> Result<AuthorizationTestDb> {
	init_test_env().await;
	let database_url = std::env::var("SERVICE_TEST_MIGRATION_DB_URL")
		.or_else(|_| std::env::var("SERVICE_DB_URL"))?;
	let schema = format!("rbac_test_{}", Uuid::new_v4().simple());
	let admin = PgPoolOptions::new()
		.max_connections(1)
		.connect(&database_url)
		.await?;
	sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
		.execute(&admin)
		.await?;
	admin.close().await;

	let connect_schema = schema.clone();
	let pool = PgPoolOptions::new()
		.max_connections(5)
		.after_connect(move |connection, _| {
			let statement = format!("SET search_path TO \"{connect_schema}\"");
			Box::pin(async move {
				sqlx::query(&statement).execute(connection).await?;
				Ok(())
			})
		})
		.connect(&database_url)
		.await?;

	let database = AuthorizationTestDb {
		pool,
		schema,
		database_url,
	};
	sqlx::query(&format!(
		"GRANT USAGE ON SCHEMA \"{}\" TO e2br3_app_role",
		database.schema
	))
	.execute(database.pool())
	.await?;
	Ok(database)
}

pub async fn apply_authorization_migrations(
	database: &AuthorizationTestDb,
) -> Result<()> {
	sqlx::raw_sql(include_str!(
		"../../../../../db/migrations/20260720_authorization_kernel.sql"
	))
	.execute(database.pool())
	.await?;
	AuthorizationMigrationService::reconcile_database(
		database.pool(),
		policy_registry(),
	)
	.await?;
	Ok(())
}

pub async fn apply_authorization_revision_migration(
	database: &AuthorizationTestDb,
) -> Result<()> {
	sqlx::raw_sql(include_str!(
		"../../../../../db/migrations/20260720_authorization_revisions.sql"
	))
	.execute(database.pool())
	.await?;
	Ok(())
}

pub async fn apply_authorization_isolation_migration(
	database: &AuthorizationTestDb,
) -> Result<()> {
	sqlx::raw_sql(include_str!(
		"../../../../../db/migrations/20260722_authorization_isolation_audit.sql"
	))
	.execute(database.pool())
	.await?;
	Ok(())
}

pub async fn scalar_i64(database: &AuthorizationTestDb, sql: &str) -> Result<i64> {
	Ok(sqlx::query_scalar(sql).fetch_one(database.pool()).await?)
}

pub async fn scalar_string(
	database: &AuthorizationTestDb,
	sql: &str,
) -> Result<String> {
	Ok(sqlx::query_scalar(sql).fetch_one(database.pool()).await?)
}
