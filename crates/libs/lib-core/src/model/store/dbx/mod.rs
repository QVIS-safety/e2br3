// region:    --- Modules

mod error;

pub use error::{Error, Result};

use crate::model::store::Db;
use sqlx::query::{Query, QueryAs};
use sqlx::{FromRow, IntoArguments, Pool, Postgres, Transaction};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::sync::Mutex;

// endregion: --- Modules

#[derive(Debug, Clone)]
pub struct Dbx {
	db_pool: Db,
	txn_state: Arc<Mutex<TxnState>>,
}

impl Dbx {
	pub fn new(db_pool: Db) -> Result<Self> {
		Ok(Dbx {
			db_pool,
			txn_state: Arc::new(Mutex::new(TxnState::Idle)),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::{Dbx, Error};
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
		let actual_database: String =
			sqlx::query_scalar("SELECT current_database()")
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
}

#[derive(Debug)]
enum TxnState {
	Idle,
	Active(TxnHolder),
	Aborted,
}

#[derive(Debug)]
struct TxnHolder {
	txn: Transaction<'static, Postgres>,
	counter: i32,
}

impl TxnHolder {
	fn new(txn: Transaction<'static, Postgres>) -> Self {
		TxnHolder { txn, counter: 1 }
	}

	fn inc(&mut self) {
		self.counter += 1;
	}

	fn dec(&mut self) -> i32 {
		self.counter -= 1;
		self.counter
	}
}

impl Deref for TxnHolder {
	type Target = Transaction<'static, Postgres>;

	fn deref(&self) -> &Self::Target {
		&self.txn
	}
}

impl DerefMut for TxnHolder {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.txn
	}
}

impl Dbx {
	pub async fn transaction_depth(&self) -> Result<i32> {
		match &*self.txn_state.lock().await {
			TxnState::Idle => Ok(0),
			TxnState::Active(txh) => Ok(txh.counter),
			TxnState::Aborted => Err(Error::TxnAborted),
		}
	}

	pub async fn begin_txn(&self) -> Result<()> {
		let mut state = self.txn_state.lock().await;
		// If we already have a tx holder, then, we increment
		match &mut *state {
			TxnState::Active(txh) => txh.inc(),
			TxnState::Idle => {
				let transaction = self.db_pool.begin().await?;
				*state = TxnState::Active(TxnHolder::new(transaction));
			}
			TxnState::Aborted => return Err(Error::TxnAborted),
		}

		Ok(())
	}

	pub async fn rollback_txn(&self) -> Result<()> {
		let mut state = self.txn_state.lock().await;
		match std::mem::replace(&mut *state, TxnState::Aborted) {
			TxnState::Active(txn_holder) => txn_holder.txn.rollback().await?,
			TxnState::Idle => *state = TxnState::Idle,
			TxnState::Aborted => {}
		}
		// No active transaction is a no-op so cleanup does not mask the original error.
		Ok(())
	}

	pub async fn commit_txn(&self) -> Result<()> {
		let mut state = self.txn_state.lock().await;
		match std::mem::replace(&mut *state, TxnState::Aborted) {
			TxnState::Active(mut txh) => {
				let counter = txh.dec();
				// If 0, then, it should be matching commit for the first first begin_txn
				// so we can commit.
				if counter == 0 {
					// here we take the txh out of the option
					txh.txn.commit().await?;
					*state = TxnState::Idle;
				} else {
					*state = TxnState::Active(txh);
				}

				Ok(())
			}
			TxnState::Idle => {
				*state = TxnState::Idle;
				Err(Error::TxnCantCommitNoOpenTxn)
			}
			TxnState::Aborted => Err(Error::TxnAborted),
		}
	}

	pub fn db(&self) -> &Pool<Postgres> {
		&self.db_pool
	}

	pub async fn fetch_one<'q, O, A>(
		&self,
		query: QueryAs<'q, Postgres, O, A>,
	) -> Result<O>
	where
		O: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
		A: IntoArguments<'q, Postgres> + 'q,
	{
		let mut state = self.txn_state.lock().await;
		let data = match std::mem::replace(&mut *state, TxnState::Aborted) {
			TxnState::Active(mut txn_holder) => {
				let res = query.fetch_one(txn_holder.as_mut()).await;
				match res {
					Ok(data) => {
						*state = TxnState::Active(txn_holder);
						data
					}
					Err(err) => {
						tracing::error!(
							"Dbx::fetch_one transaction query failed: {err}"
						);
						let _ = txn_holder.txn.rollback().await;
						return Err(err.into());
					}
				}
			}
			TxnState::Idle => {
				*state = TxnState::Idle;
				query.fetch_one(self.db()).await?
			}
			TxnState::Aborted => return Err(Error::TxnAborted),
		};

		Ok(data)
	}

	pub async fn fetch_optional<'q, O, A>(
		&self,
		query: QueryAs<'q, Postgres, O, A>,
	) -> Result<Option<O>>
	where
		O: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
		A: IntoArguments<'q, Postgres> + 'q,
	{
		let mut state = self.txn_state.lock().await;
		let data = match std::mem::replace(&mut *state, TxnState::Aborted) {
			TxnState::Active(mut txn_holder) => {
				let res = query.fetch_optional(txn_holder.as_mut()).await;
				match res {
					Ok(data) => {
						*state = TxnState::Active(txn_holder);
						data
					}
					Err(err) => {
						tracing::error!(
							"Dbx::fetch_optional transaction query failed: {err}"
						);
						let _ = txn_holder.txn.rollback().await;
						return Err(err.into());
					}
				}
			}
			TxnState::Idle => {
				*state = TxnState::Idle;
				query.fetch_optional(self.db()).await?
			}
			TxnState::Aborted => return Err(Error::TxnAborted),
		};

		Ok(data)
	}

	pub async fn fetch_all<'q, O, A>(
		&self,
		query: QueryAs<'q, Postgres, O, A>,
	) -> Result<Vec<O>>
	where
		O: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
		A: IntoArguments<'q, Postgres> + 'q,
	{
		let mut state = self.txn_state.lock().await;
		let data = match std::mem::replace(&mut *state, TxnState::Aborted) {
			TxnState::Active(mut txn_holder) => {
				let res = query.fetch_all(txn_holder.as_mut()).await;
				match res {
					Ok(data) => {
						*state = TxnState::Active(txn_holder);
						data
					}
					Err(err) => {
						tracing::error!(
							"Dbx::fetch_all transaction query failed: {err}"
						);
						let _ = txn_holder.txn.rollback().await;
						return Err(err.into());
					}
				}
			}
			TxnState::Idle => {
				*state = TxnState::Idle;
				query.fetch_all(self.db()).await?
			}
			TxnState::Aborted => return Err(Error::TxnAborted),
		};

		Ok(data)
	}

	pub async fn execute<'q, A>(&self, query: Query<'q, Postgres, A>) -> Result<u64>
	where
		A: IntoArguments<'q, Postgres> + 'q,
	{
		let mut state = self.txn_state.lock().await;
		let row_affected = match std::mem::replace(&mut *state, TxnState::Aborted) {
			TxnState::Active(mut txn_holder) => {
				let res = query.execute(txn_holder.as_mut()).await;
				match res {
					Ok(done) => {
						*state = TxnState::Active(txn_holder);
						done.rows_affected()
					}
					Err(err) => {
						tracing::error!(
							"Dbx::execute transaction query failed: {err}"
						);
						let _ = txn_holder.txn.rollback().await;
						return Err(err.into());
					}
				}
			}
			TxnState::Idle => {
				*state = TxnState::Idle;
				query.execute(self.db()).await?.rows_affected()
			}
			TxnState::Aborted => return Err(Error::TxnAborted),
		};

		Ok(row_affected)
	}
}
