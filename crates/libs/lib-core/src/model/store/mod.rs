// region:    --- Modules

pub(in crate::model) mod dbx;

use crate::core_config;
use crate::ctx::{Ctx, ROLE_ADB_ADMIN};
use crate::model::Error;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

// endregion: --- Modules

pub type Db = Pool<Postgres>;

pub async fn new_db_pool() -> sqlx::Result<Db> {
	// * See NOTE 1) below
	let max_connections = if cfg!(test) { 1 } else { 5 };

	PgPoolOptions::new()
		.max_connections(max_connections)
		.after_connect(|conn, _meta| {
			Box::pin(async move {
				sqlx::query("SET ROLE e2br3_app_role")
					.execute(&mut *conn)
					.await?;
				Ok(())
			})
		})
		.connect(&core_config().DB_URL)
		.await
}

// region:    --- User Context Helpers

/// Sets the current user context for PostgreSQL session.
/// This enables audit triggers to capture user_id for regulatory compliance.
///
/// CRITICAL: This must be called at the start of every transaction that performs
/// INSERT, UPDATE, or DELETE operations to ensure audit trail compliance with
/// 21 CFR Part 11, EMA GVP Module VI, and ALCOA+ principles.
pub async fn set_user_context(
	tx: &mut Transaction<'_, Postgres>,
	user_id: Uuid,
) -> Result<(), Error> {
	sqlx::query("SELECT set_current_user_context($1)")
		.bind(user_id)
		.execute(&mut **tx)
		.await
		.map_err(|e| Error::Store(format!("Failed to set user context: {e}")))?;

	Ok(())
}

/// Sets the current user context using Dbx (respects existing transactions).
pub async fn set_user_context_dbx(
	dbx: &dbx::Dbx,
	user_id: Uuid,
) -> Result<(), Error> {
	let query = sqlx::query("SELECT set_current_user_context($1)").bind(user_id);
	dbx.execute(query)
		.await
		.map_err(|e| Error::Store(format!("Failed to set user context: {e}")))?;

	Ok(())
}

// region:    --- Organization Context for RLS

/// Sets the organization context for Row-Level Security (RLS).
/// This enables the database to enforce organization isolation.
///
/// Call this at the start of each request to set up RLS context.
#[allow(dead_code)]
pub async fn set_org_context(
	tx: &mut Transaction<'_, Postgres>,
	organization_id: Uuid,
	role: &str,
) -> Result<(), Error> {
	let db_role = canonical_db_role(role);
	sqlx::query("SELECT set_org_context($1, $2)")
		.bind(organization_id)
		.bind(db_role)
		.execute(&mut **tx)
		.await
		.map_err(|e| Error::Store(format!("Failed to set org context: {e}")))?;

	Ok(())
}

/// Sets the organization context using Dbx (for non-transactional queries).
#[allow(dead_code)]
pub async fn set_org_context_dbx(
	dbx: &dbx::Dbx,
	organization_id: Uuid,
	role: &str,
) -> Result<(), Error> {
	let db_role = canonical_db_role(role);
	let query = sqlx::query("SELECT set_org_context($1, $2)")
		.bind(organization_id)
		.bind(db_role);
	dbx.execute(query)
		.await
		.map_err(|e| Error::Store(format!("Failed to set org context: {e}")))?;

	Ok(())
}

fn canonical_db_role(role: &str) -> &str {
	if role == ROLE_ADB_ADMIN {
		"admin"
	} else {
		role
	}
}

/// Sets optional compliance context for audit enrichment.
/// Values are transaction-scoped and read by DB audit triggers.
pub async fn set_compliance_context_dbx(
	dbx: &dbx::Dbx,
	change_reason: Option<&str>,
	e_signature_id: Option<Uuid>,
) -> Result<(), Error> {
	let reason = change_reason.unwrap_or("");
	let sig = e_signature_id.map(|id| id.to_string()).unwrap_or_default();

	let query = sqlx::query(
		"SELECT set_config('app.change_reason', $1, true),
		        set_config('app.e_signature_id', $2, true)",
	)
	.bind(reason)
	.bind(sig);
	dbx.execute(query).await.map_err(|e| {
		Error::Store(format!("Failed to set compliance context: {e}"))
	})?;
	Ok(())
}

/// Sets both user context (for audit trail) and organization context (for RLS).
/// This is the recommended function to call at the start of each request.
#[allow(dead_code)]
pub async fn set_full_context_dbx(
	dbx: &dbx::Dbx,
	user_id: Uuid,
	organization_id: Uuid,
	role: &str,
) -> Result<(), Error> {
	// Set user context for audit trail
	set_user_context_dbx(dbx, user_id).await?;
	// Set organization context for RLS
	set_org_context_dbx(dbx, organization_id, role).await?;
	Ok(())
}

/// Sets full context from Ctx, including optional compliance context.
pub async fn set_full_context_from_ctx_dbx(
	dbx: &dbx::Dbx,
	ctx: &Ctx,
) -> Result<(), Error> {
	set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
		.await?;
	set_compliance_context_dbx(dbx, ctx.change_reason(), ctx.e_signature_id())
		.await?;
	Ok(())
}

/// Sets full context, rolling back the active transaction if it fails.
pub async fn set_full_context_dbx_or_rollback(
	dbx: &dbx::Dbx,
	user_id: Uuid,
	organization_id: Uuid,
	role: &str,
) -> Result<(), Error> {
	if let Err(err) = set_full_context_dbx(dbx, user_id, organization_id, role).await
	{
		let _ = dbx.rollback_txn().await;
		return Err(err);
	}
	Ok(())
}

// endregion: --- Organization Context for RLS

/// Gets the current user context from PostgreSQL session.
/// Used for verification and debugging purposes.
#[allow(dead_code)]
pub async fn get_user_context(
	tx: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, Error> {
	let row: (Uuid,) = sqlx::query_as("SELECT get_current_user_context()")
		.fetch_one(&mut **tx)
		.await
		.map_err(|e| Error::Store(format!("Failed to get user context: {e}")))?;

	Ok(row.0)
}

// endregion: --- User Context Helpers

// NOTE 1) This is not an ideal situation; however, with sqlx 0.7.1, when executing `cargo test`, some tests that use sqlx fail at a
//         rather low level (in the tokio scheduler). It appears to be a low-level thread/async issue, as removing/adding
//         tests causes different tests to fail. The cause remains uncertain, but setting max_connections to 1 resolves the issue.
//         The good news is that max_connections still function normally for a regular run.
//         This issue is likely due to the unique requirements unit tests impose on their execution, and therefore,
//         while not ideal, it should serve as an acceptable temporary solution.
//         It's a very challenging issue to investigate and narrow down. The alternative would have been to stick with sqlx 0.6.x, which
//         is potentially less ideal and might lead to confusion as to why we are maintaining the older version in this blueprint.
