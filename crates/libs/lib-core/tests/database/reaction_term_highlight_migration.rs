use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn isolated_pool() -> TestResult<(PgPool, PgPool, String)> {
	let database_url = std::env::var("SERVICE_DB_URL")?;
	let admin = PgPoolOptions::new()
		.max_connections(1)
		.connect(&database_url)
		.await?;
	let schema = format!("reaction_highlight_migration_{}", Uuid::new_v4().simple());
	admin
		.execute(sqlx::query(&format!("CREATE SCHEMA \"{schema}\"")))
		.await?;
	let connect_schema = schema.clone();
	let pool = PgPoolOptions::new()
		.max_connections(1)
		.after_connect(move |connection, _| {
			let statement = format!("SET search_path TO \"{connect_schema}\"");
			Box::pin(async move {
				sqlx::query(&statement).execute(&mut *connection).await?;
				Ok(())
			})
		})
		.connect(&database_url)
		.await?;
	Ok((admin, pool, schema))
}

async fn drop_isolated_schema(
	admin: PgPool,
	pool: PgPool,
	schema: &str,
) -> TestResult {
	pool.close().await;
	admin
		.execute(sqlx::query(&format!("DROP SCHEMA \"{schema}\" CASCADE")))
		.await?;
	admin.close().await;
	Ok(())
}

#[tokio::test]
async fn migration_accepts_fresh_varchar_bootstrap_schema() -> TestResult {
	let (admin, pool, schema) = isolated_pool().await?;
	sqlx::raw_sql(
		"
		CREATE TABLE reactions (
			id uuid PRIMARY KEY,
			term_highlighted VARCHAR(1)
				CHECK (term_highlighted IN ('1', '2', '3', '4'))
		);
		INSERT INTO reactions (id, term_highlighted)
		VALUES (gen_random_uuid(), '1'), (gen_random_uuid(), '4');
		",
	)
	.execute(&pool)
	.await?;

	let migration = sqlx::raw_sql(include_str!(
		"../../../../../db/migrations/20260725_reaction_term_highlight_code.sql"
	))
	.execute(&pool)
	.await;
	let rerun = if migration.is_ok() {
		Some(
			sqlx::raw_sql(include_str!(
			"../../../../../db/migrations/20260725_reaction_term_highlight_code.sql"
		))
			.execute(&pool)
			.await,
		)
	} else {
		None
	};
	let values = if migration.is_ok() {
		sqlx::query_scalar::<_, Option<String>>(
			"SELECT term_highlighted FROM reactions ORDER BY term_highlighted",
		)
		.fetch_all(&pool)
		.await?
	} else {
		Vec::new()
	};

	drop_isolated_schema(admin, pool, &schema).await?;
	migration?;
	if let Some(rerun) = rerun {
		rerun?;
	}
	assert_eq!(values, vec![Some("1".to_string()), Some("4".to_string())]);
	Ok(())
}

#[tokio::test]
async fn migration_converts_legacy_boolean_values() -> TestResult {
	let (admin, pool, schema) = isolated_pool().await?;
	sqlx::raw_sql(
		"
		CREATE TABLE reactions (
			id uuid PRIMARY KEY,
			term_highlighted BOOLEAN
		);
		INSERT INTO reactions (id, term_highlighted)
		VALUES
			(gen_random_uuid(), TRUE),
			(gen_random_uuid(), FALSE),
			(gen_random_uuid(), NULL);
		",
	)
	.execute(&pool)
	.await?;

	let migration = sqlx::raw_sql(include_str!(
		"../../../../../db/migrations/20260725_reaction_term_highlight_code.sql"
	))
	.execute(&pool)
	.await;
	let values = if migration.is_ok() {
		sqlx::query_scalar::<_, Option<String>>(
			"SELECT term_highlighted FROM reactions ORDER BY term_highlighted NULLS LAST",
		)
		.fetch_all(&pool)
		.await?
	} else {
		Vec::new()
	};

	drop_isolated_schema(admin, pool, &schema).await?;
	migration?;
	assert_eq!(
		values,
		vec![Some("1".to_string()), Some("2".to_string()), None]
	);
	Ok(())
}
