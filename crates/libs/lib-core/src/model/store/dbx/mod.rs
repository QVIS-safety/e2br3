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
