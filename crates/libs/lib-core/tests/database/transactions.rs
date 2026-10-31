use lib_core::model::store::dbx::{Dbx, Error};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

async fn test_dbx() -> Dbx {
	let expected_database = std::env::var("E2BR3_TEST_DATABASE_NAME")
		.expect("Dbx tests require scripts/test-isolated-db.sh");
	assert!(!expected_database.trim().is_empty());
	let database_url = std::env::var("SERVICE_DB_URL")
		.expect("SERVICE_DB_URL is required for Dbx tests");
	let pool = PgPoolOptions::new()
		.max_connections(3)
		.connect(&database_url)
		.await
		.expect("connect to isolated test database");
	let actual_database: String = sqlx::query_scalar("SELECT current_database()")
		.fetch_one(&pool)
		.await
		.expect("read current test database");
	assert_eq!(actual_database, expected_database);
	Dbx::new(pool).expect("test Dbx")
}

async fn reset_test_rows(dbx: &Dbx) {
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS dbx_transaction_safety_test (id integer PRIMARY KEY)",
	)
	.execute(dbx.db())
	.await
	.expect("create transaction test table");
	sqlx::query("DELETE FROM dbx_transaction_safety_test")
		.execute(dbx.db())
		.await
		.expect("clear transaction test rows");
}

async fn test_row_count(dbx: &Dbx) -> i64 {
	sqlx::query_scalar("SELECT COUNT(*) FROM dbx_transaction_safety_test")
		.fetch_one(dbx.db())
		.await
		.expect("count transaction test rows")
}

#[serial_test::serial]
#[tokio::test]
async fn nested_rollback_aborts_dbx() {
	let dbx = test_dbx().await;
	reset_test_rows(&dbx).await;
	dbx.begin_txn().await.expect("outer transaction");
	dbx.execute(sqlx::query(
		"INSERT INTO dbx_transaction_safety_test (id) VALUES (1)",
	))
	.await
	.expect("outer write");
	dbx.begin_txn().await.expect("nested transaction");
	dbx.execute(sqlx::query(
		"INSERT INTO dbx_transaction_safety_test (id) VALUES (2)",
	))
	.await
	.expect("nested write");
	dbx.rollback_txn().await.expect("rollback");

	assert_eq!(test_row_count(&dbx).await, 0);
	assert!(matches!(dbx.commit_txn().await, Err(Error::TxnAborted)));
	assert!(matches!(dbx.begin_txn().await, Err(Error::TxnAborted)));
	assert!(matches!(
		dbx.fetch_one(sqlx::query_as::<_, (i32,)>("SELECT 1")).await,
		Err(Error::TxnAborted)
	));
}

#[serial_test::serial]
#[tokio::test]
async fn balanced_nested_commit_keeps_dbx_usable() {
	let dbx = test_dbx().await;
	dbx.begin_txn().await.expect("outer transaction");
	dbx.begin_txn().await.expect("nested transaction");
	dbx.commit_txn().await.expect("nested commit");
	let (value,) = dbx
		.fetch_one(sqlx::query_as::<_, (i32,)>("SELECT 1"))
		.await
		.expect("transaction query");
	assert_eq!(value, 1);
	dbx.commit_txn().await.expect("outer commit");

	let (value,) = dbx
		.fetch_one(sqlx::query_as::<_, (i32,)>("SELECT 2"))
		.await
		.expect("pool query");
	assert_eq!(value, 2);
}

#[serial_test::serial]
#[tokio::test]
async fn transaction_query_failure_aborts_dbx() {
	let dbx = test_dbx().await;
	reset_test_rows(&dbx).await;
	dbx.begin_txn().await.expect("transaction");
	dbx.execute(sqlx::query(
		"INSERT INTO dbx_transaction_safety_test (id) VALUES (1)",
	))
	.await
	.expect("write before failure");
	assert!(matches!(
		dbx.execute(sqlx::query("SELECT * FROM dbx_missing_table"))
			.await,
		Err(Error::Sqlx(_))
	));
	assert_eq!(test_row_count(&dbx).await, 0);

	assert!(matches!(
		dbx.execute(sqlx::query("SELECT 1")).await,
		Err(Error::TxnAborted)
	));
}

#[serial_test::serial]
#[tokio::test]
async fn cancelled_transaction_query_aborts_dbx() {
	let dbx = test_dbx().await;
	reset_test_rows(&dbx).await;
	dbx.begin_txn().await.expect("transaction");
	dbx.execute(sqlx::query(
		"INSERT INTO dbx_transaction_safety_test (id) VALUES (1)",
	))
	.await
	.expect("write before cancellation");
	let mut blocker = dbx.db().acquire().await.expect("lock connection");
	sqlx::query("SELECT pg_advisory_lock(8675309)")
		.execute(&mut *blocker)
		.await
		.expect("advisory lock");
	let query_dbx = dbx.clone();
	let query = tokio::spawn(async move {
		query_dbx
			.fetch_one(sqlx::query_as::<_, (i32,)>(
				"SELECT 1 FROM pg_advisory_xact_lock(8675309) /* dbx_cancel_test */",
			))
			.await
	});
	tokio::time::timeout(Duration::from_secs(5), async {
		loop {
			let waiting: bool = sqlx::query_scalar(
				"SELECT EXISTS (
					SELECT 1 FROM pg_stat_activity
					WHERE query LIKE '%dbx_cancel_test%'
					  AND wait_event = 'advisory'
				)",
			)
			.fetch_one(dbx.db())
			.await
			.expect("inspect blocked query");
			if waiting {
				break;
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	})
	.await
	.expect("transaction query reached advisory lock");
	query.abort();
	assert!(query.await.expect_err("cancelled query").is_cancelled());
	sqlx::query("SELECT pg_advisory_unlock(8675309)")
		.execute(&mut *blocker)
		.await
		.expect("advisory unlock");

	assert_eq!(test_row_count(&dbx).await, 0);
	assert!(matches!(dbx.begin_txn().await, Err(Error::TxnAborted)));
	assert!(matches!(
		dbx.fetch_one(sqlx::query_as::<_, (i32,)>("SELECT 1")).await,
		Err(Error::TxnAborted)
	));
}
